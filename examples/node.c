#include <errno.h>
#include <stdio.h>
#include <math.h>
#include <sys/mman.h>
 
#include <spa/utils/result.h>
#include <spa/param/audio/format-utils.h>
#include <spa/param/props.h>
#include <spa/node/io.h>
#include <spa/node/utils.h>
#include <spa/pod/filter.h>
#include <spa/debug/format.h>
#include <spa/debug/pod.h>
 
#include <pipewire/pipewire.h>
#include <pipewire/impl-factory.h>
 
#define M_PI_M2f (float)(M_PI + M_PI)
 
#define DSP_RATE        44100
#define BUFFER_SAMPLES  128
#define MAX_BUFFERS     32
#define SINE_FREQ       440.0
#define VOLUME          0.2

PW_LOG_TOPIC_STATIC(livemix, "livemix");
#define PW_LOG_TOPIC_DEFAULT livemix
 
struct buffer {
        uint32_t id;
        struct spa_buffer *buffer;
        struct spa_list link;
        void *ptr;
        bool mapped;
};

struct port {
        struct spa_audio_info_raw format;

        struct spa_port_info port_info;
        struct pw_properties *port_props;
#define PORT_EnumFormat	0
#define PORT_Meta	1
#define PORT_IO		2
#define PORT_Format	3
#define PORT_Buffers	4
#define N_PORT_PARAMS	5
        struct spa_param_info port_params[N_PORT_PARAMS];

        struct buffer buffers[MAX_BUFFERS];
        uint32_t n_buffers;
        struct spa_io_buffers *io;
        struct spa_io_control *io_notify;
        uint32_t io_notify_size;

        float accumulator;
        float volume_accum;
 
        struct spa_list empty;
};

struct data {
        const char *path;
 
        struct pw_main_loop *loop;
 
        struct pw_context *context;
 
        struct pw_core *core;
        struct spa_hook core_listener;

        uint64_t change_mask_all;
        struct pw_properties *props;
	struct spa_node_info info;
#define NODE_PropInfo		0
#define NODE_Props		1
#define NODE_EnumFormat		2
#define NODE_Format		3
#define N_NODE_PARAMS		4
        struct spa_param_info params[N_NODE_PARAMS];

        uint64_t port_change_mask_all;
#define MAX_PORTS 16
        struct port ports[MAX_PORTS];
        uint32_t n_ports;
 
        struct spa_node impl_node;
        struct spa_hook_list hooks;

	struct pw_impl_node *node;
	struct pw_impl_node *adapter_node;
	struct spa_hook node_listener;
	struct spa_hook node_rt_listener;

	struct pw_proxy *proxy;
	struct spa_hook proxy_listener;
};

static void emit_node_info(struct data *d, bool full) {
	uint32_t i;
	uint64_t old = full ? d->info.change_mask : 0;
	if (full) {
		d->info.change_mask = d->change_mask_all;
        }

	if (d->info.change_mask != 0) {
		if (d->info.change_mask & SPA_NODE_CHANGE_MASK_PARAMS) {
			for (i = 0; i < d->info.n_params; i++) {
				if (d->params[i].user > 0) {
					d->params[i].flags ^= SPA_PARAM_INFO_SERIAL;
					d->params[i].user = 0;
				}
			}
		}

		spa_node_emit_info(&d->hooks, &d->info);
	}

	d->info.change_mask = old;
}

static void emit_port_info(struct data *d, bool full) {
	uint32_t i;
	uint64_t old;
        int n;
        struct port *port;
        
        for (n = 0; n < d->n_ports; n++) {
                port = &d->ports[n];
                old = full ? port->port_info.change_mask : 0;

                if (full) {
                        port->port_info.change_mask = d->port_change_mask_all;
                }

                if (port->port_info.change_mask != 0) {
                        if (port->port_info.change_mask & SPA_PORT_CHANGE_MASK_PARAMS) {
                                for (i = 0; i < port->port_info.n_params; i++) {
                                        if (port->port_params[i].user > 0) {
                                                port->port_params[i].flags ^= SPA_PARAM_INFO_SERIAL;
                                                port->port_params[i].user = 0;
                                        }
                                }
                        }

                        spa_node_emit_port_info(&d->hooks, SPA_DIRECTION_OUTPUT, n, &port->port_info);
                }

                port->port_info.change_mask = old;
        }
}

static void port_update_volume(struct port *p)
{
        struct spa_pod_builder b = { 0 };
        struct spa_pod_frame f[2];
 
        if (p->io_notify == NULL) {
                return;
        }
 
        spa_pod_builder_init(&b, p->io_notify, p->io_notify_size);
        spa_pod_builder_push_sequence(&b, &f[0], 0);
        spa_pod_builder_control(&b, 0, SPA_CONTROL_Properties);
        spa_pod_builder_push_object(&b, &f[1], SPA_TYPE_OBJECT_Props, 0);
        spa_pod_builder_prop(&b, SPA_PROP_volume, 0);
        spa_pod_builder_float(&b, (sinf(p->volume_accum) / 2.0f) + 0.5f);
        spa_pod_builder_pop(&b, &f[1]);
        spa_pod_builder_pop(&b, &f[0]);
 
        p->volume_accum += M_PI_M2f / 1000.0f;

        if (p->volume_accum >= M_PI_M2f) {
                p->volume_accum -= M_PI_M2f;
        }
}
 
static int impl_send_command(void *object, const struct spa_command *command)
{
        pw_log_info("send_command");
        return 0;
}
 
static int impl_add_listener(void *object,
                struct spa_hook *listener,
                const struct spa_node_events *events,
                void *data)
{
        struct data *d = object;
        struct spa_hook_list save;
 
        spa_hook_list_isolate(&d->hooks, &save, listener, events, data);

	emit_node_info(d, true);
	emit_port_info(d, true);
 
        spa_hook_list_join(&d->hooks, &save);

        return 0;
}
 
static int impl_set_callbacks(void *object,
                              const struct spa_node_callbacks *callbacks, void *data)
{
        pw_log_info("set_callbacks");
        return 0;
}

static int impl_enum_params(void *object, int seq, uint32_t id, uint32_t start, uint32_t num,
				 const struct spa_pod *filter)
{
        struct data *d = object;
        pw_log_trace("%p: enum params %d (%s) seq:%d",
                d, id, spa_debug_type_find_name(spa_type_param, id), seq);
        return -ENOENT;
}

static int impl_set_param(void *object, uint32_t id, uint32_t flags, const struct spa_pod *param) {
	struct data *d = object;

        pw_log_info("%p: set param id %d (%s) flags:%d",
                d, id, spa_debug_type_find_name(spa_type_param, id), flags);

        if (param != NULL) {
                spa_debug_pod(0, NULL, param);
        }

	emit_node_info(d, false);
	emit_port_info(d, false);
        return 0;
}

static int impl_set_io(void *object, uint32_t id, void *data, size_t size) {
        pw_log_info("set_io id:%d, size:%zu", id, size);
        return 0;
}
 
static int impl_port_set_io(
        void *object, enum spa_direction direction, uint32_t port_id, uint32_t id, void *data, size_t size
) {
        pw_log_info("port_set_io direction:%d, port_id:%d, id:%d, size:%zu", direction, port_id, id, size);

        struct data *d = object;

        if (port_id >= d->n_ports) {
                pw_log_error("%p: invalid port id %d", d, port_id);
                return -EINVAL;
        }

        struct port *p = &d->ports[port_id];
 
        switch (id) {
        case SPA_IO_Buffers:
                p->io = data;
                break;
        case SPA_IO_Notify:
                p->io_notify = data;
                p->io_notify_size = size;
                break;
        default:
                return -ENOENT;
        }

        return 0;
}

static int impl_port_enum_params(void *object, int seq,
                                 enum spa_direction direction, uint32_t port_id,
                                 uint32_t id, uint32_t start, uint32_t num,
                                 const struct spa_pod *filter)
{
        struct data *d = object;
        struct spa_result_node_params result;
        struct spa_pod *param = NULL;
        struct spa_pod_builder b = { 0 };
        uint8_t buffer[1024];
        int emitted = 0;
        
        if (port_id >= d->n_ports) {
                pw_log_error("%p: invalid port id %d", d, port_id);
                return -EINVAL;
        }

        struct port *p = &d->ports[port_id];

        pw_log_info("%p: port_enum_params id %d (%s) start:%d num:%d port_id:%d",
                d, id, spa_debug_type_find_name(spa_type_param, id), start, num, port_id);

        if (filter != NULL) {
                spa_debug_pod(0, NULL, filter);
        }

        result.id = id;
        result.next = start;

        while (true) {
                param = NULL;
                
                if (emitted >= num) {
                        break;
                }

                result.index = result.next++;

                spa_pod_builder_init(&b, buffer, sizeof(buffer));

                switch (id) {
                case SPA_PARAM_EnumFormat:
                        switch (result.index) {
                        case 0:
                                param = spa_pod_builder_add_object(&b,
                                        SPA_TYPE_OBJECT_Format, SPA_PARAM_EnumFormat,
                                        SPA_FORMAT_mediaType,      SPA_POD_Id(SPA_MEDIA_TYPE_audio),
                                        SPA_FORMAT_mediaSubtype,   SPA_POD_Id(SPA_MEDIA_SUBTYPE_raw),
                                        SPA_FORMAT_AUDIO_format,   SPA_POD_CHOICE_ENUM_Id(5,
                                                                        SPA_AUDIO_FORMAT_S16,
                                                                        SPA_AUDIO_FORMAT_S16P,
                                                                        SPA_AUDIO_FORMAT_S16,
                                                                        SPA_AUDIO_FORMAT_F32P,
                                                                        SPA_AUDIO_FORMAT_F32),
                                        SPA_FORMAT_AUDIO_channels, SPA_POD_CHOICE_RANGE_Int(2, 1, INT32_MAX),
                                        SPA_FORMAT_AUDIO_rate,     SPA_POD_CHOICE_RANGE_Int(44100, 1, INT32_MAX));
                                break;
                        default:
                                return 0;
                        }

                        break;
                case SPA_PARAM_Format:
                        switch (result.index) {
                        case 0:
                                if (p->format.format == SPA_AUDIO_FORMAT_UNKNOWN) {
                                        return 0;
                                }

                                param = spa_format_audio_raw_build(&b, id, &p->format);
                                break;
                        default:
                                return 0;
                        }

                        break;
                case SPA_PARAM_Buffers:
                        switch (result.index) {
                        case 0:
                                param = spa_pod_builder_add_object(&b,
                                        SPA_TYPE_OBJECT_ParamBuffers, id,
                                        SPA_PARAM_BUFFERS_buffers, SPA_POD_CHOICE_RANGE_Int(1, 1, 32),
                                        SPA_PARAM_BUFFERS_blocks,  SPA_POD_Int(1),
                                        SPA_PARAM_BUFFERS_size,    SPA_POD_CHOICE_RANGE_Int(
                                                                        BUFFER_SAMPLES * sizeof(float), 32, INT32_MAX),
                                        SPA_PARAM_BUFFERS_stride,  SPA_POD_Int(sizeof(float)));

                                break;
                        default:
                                return 0;
                        }

                        break;
                case SPA_PARAM_Meta:
                        switch (result.index) {
                        case 0:
                                param = spa_pod_builder_add_object(&b,
                                        SPA_TYPE_OBJECT_ParamMeta, id,
                                        SPA_PARAM_META_type, SPA_POD_Id(SPA_META_Header),
                                        SPA_PARAM_META_size, SPA_POD_Int(sizeof(struct spa_meta_header)));
                                break;
                        default:
                                return 0;
                        }

                        break;
                case SPA_PARAM_IO:
                        switch (result.index) {
                        case 0:
                                param = spa_pod_builder_add_object(&b,
                                        SPA_TYPE_OBJECT_ParamIO, id,
                                        SPA_PARAM_IO_id, SPA_POD_Id(SPA_IO_Buffers),
                                        SPA_PARAM_IO_size, SPA_POD_Int(sizeof(struct spa_io_buffers)));
                                break;
                        case 1:
                                param = spa_pod_builder_add_object(&b,
                                        SPA_TYPE_OBJECT_ParamIO, id,
                                        SPA_PARAM_IO_id, SPA_POD_Id(SPA_IO_Notify),
                                        SPA_PARAM_IO_size, SPA_POD_Int(sizeof(struct spa_io_sequence) + 1024));
                                break;
                        default:
                                return 0;
                        }

                        break;
                default:
                        return -ENOENT;
                }

                if (spa_pod_filter(&b, &result.param, param, filter) < 0) {
                        pw_log_warn("filter failed");
                        continue;
                }

                spa_node_emit_result(&d->hooks, seq, 0, SPA_RESULT_TYPE_NODE_PARAMS, &result);
                emitted += 1;
        }

        return 0;
}

static int impl_port_set_param(void *object,
                               enum spa_direction direction, uint32_t port_id,
                               uint32_t id, uint32_t flags,
                               const struct spa_pod *param)
{
        struct data *d = object;

        if (port_id >= d->n_ports) {
                return -EINVAL;
        }

        struct port *p = &d->ports[port_id];

        pw_log_info("%p: port_set_param %d (%s) direction:%d, port_id:%d, flags:%d",
                d, id, spa_debug_type_find_name(spa_type_param, id), direction, port_id, flags);

        if (param != NULL) {
                spa_debug_pod(0, NULL, param);
        }

        switch (id) {
        case SPA_PARAM_Format:
                if (param != NULL) {
                        spa_format_audio_raw_parse(param, &p->format);
                        d->params[PORT_Format] = SPA_PARAM_INFO(SPA_PARAM_Format, SPA_PARAM_INFO_READWRITE);
                        d->params[PORT_Buffers] = SPA_PARAM_INFO(SPA_PARAM_Buffers, SPA_PARAM_INFO_READ);
                } else {
                        spa_zero(p->format);
                        d->params[PORT_Format] = SPA_PARAM_INFO(SPA_PARAM_Format, SPA_PARAM_INFO_WRITE);
                        d->params[PORT_Buffers] = SPA_PARAM_INFO(SPA_PARAM_Buffers, 0);
                }

                break;
        default:
                return -ENOENT;
        }

        p->port_info.change_mask = SPA_PORT_CHANGE_MASK_PARAMS;
        
	emit_node_info(d, false);
	emit_port_info(d, false);
        return 0;
}

static int impl_port_use_buffers(void *object,
                enum spa_direction direction, uint32_t port_id,
                uint32_t flags,
                struct spa_buffer **buffers, uint32_t n_buffers)
{
        struct data *d = object;

        pw_log_info("port_use_buffers direction:%d, port_id:%d, flags:%d, n_buffers:%d", direction, port_id, flags, n_buffers);

        if (port_id >= d->n_ports) {
                return -EINVAL;
        }

        struct port *p = &d->ports[port_id];
 
        if (n_buffers > MAX_BUFFERS) {
                return -ENOSPC;
        }
 
        for (uint32_t i = 0; i < n_buffers; i++) {
                struct buffer *b = &p->buffers[i];
                struct spa_data *datas = buffers[i]->datas;
 
                if (datas[0].data != NULL) {
                        b->ptr = datas[0].data;
                        b->mapped = false;
                }
                else if (datas[0].type == SPA_DATA_MemFd ||
                         datas[0].type == SPA_DATA_DmaBuf) {
                        b->ptr = mmap(NULL, datas[0].maxsize, PROT_WRITE,
                                      MAP_SHARED, datas[0].fd, datas[0].mapoffset);
                        if (b->ptr == MAP_FAILED) {
                                pw_log_error("failed to buffer mem");
                                return -errno;
 
                        }
                        b->mapped = true;
                }
                else {
                        pw_log_error("invalid buffer mem");
                        return -EINVAL;
                }
                b->id = i;
                b->buffer = buffers[i];
                pw_log_debug("got buffer %d size %d", i, datas[0].maxsize);
                spa_list_append(&p->empty, &b->link);
        }

        p->n_buffers = n_buffers;
        return 0;
}
 
static inline void port_reuse_buffer(struct port *p, uint32_t id)
{
        pw_log_info("port_reuse_buffer: %p: recycle buffer %d", p, id);
        spa_list_append(&p->empty, &p->buffers[id].link);
}

static int impl_port_reuse_buffer(void *object, uint32_t port_id, uint32_t buffer_id)
{
        struct data *d = object;

        if (port_id >= d->n_ports) {
                pw_log_error("%p: invalid port id %d", d, port_id);
                return -EINVAL;
        }

        struct port *p = &d->ports[port_id];
        port_reuse_buffer(p, buffer_id);
        return 0;
}

/*
static void fill_s16(struct data *d, void *dest, int avail)
{
        pw_log_trace("fill_s16 channels=%d, rate=%d, avail=%d", d->format.channels, d->format.rate, avail);

        int16_t *dst = dest;
        int n_samples = avail / (sizeof(int16_t) * d->format.channels);
        int i;
        uint32_t c;
 
        for (i = 0; i < n_samples; i++) {
                int16_t val;
 
                d->accumulator += (M_PI_M2f * SINE_FREQ) / d->format.rate;

                if (d->accumulator >= M_PI_M2f)
                        d->accumulator -= M_PI_M2f;
 
                val = (int16_t) (sinf(d->accumulator) * VOLUME * 32767.0f);
 
                for (c = 0; c < d->format.channels; c++)
                        *dst++ = val;
        }
}

static void fill_f32(struct data *d, void *dest, int avail)
{
        pw_log_trace("fill_f32 channels=%d, rate=%d, avail=%d", d->format.channels, d->format.rate, avail);

        float *dst = dest;
        int n_samples = avail / (sizeof(float) * d->format.channels);
        int i;
        uint32_t c;
 
        for (i = 0; i < n_samples; i++) {
                float val;
 
                d->accumulator += (M_PI_M2f * SINE_FREQ) / (float) d->format.rate;

                if (d->accumulator >= M_PI_M2f)
                        d->accumulator -= M_PI_M2f;
 
                val = sinf(d->accumulator) * VOLUME;
 
                for (c = 0; c < d->format.channels; c++) {
                        *dst++ = val;
                }
        }
}

static void fill_f32_planar(struct data *d, void *dest, int avail)
{
        float *dst = dest;
        int n_samples = avail / sizeof(float);
        int i;

        pw_log_info("fill_f32_planar channels=%d, rate=%d, avail=%d, n_samples=%d", d->format.channels, d->format.rate, avail, n_samples);

        for (i = 0; i < n_samples; i++) {
                float val;
 
                d->accumulator += (M_PI_M2f * SINE_FREQ) / (float) d->format.rate;

                if (d->accumulator >= M_PI_M2f)
                        d->accumulator -= M_PI_M2f;
 
                val = sinf(d->accumulator) * VOLUME;
                *dst = val;
                dst += 1;
        }
}
*/

static int impl_node_process(void *object)
{
        pw_log_warn("impl_node_process");

        struct data *d = object;
        struct buffer *b;
        uint32_t maxsize;
        struct spa_data *od;
        int n = 0;

        for (n = 0; n < d->n_ports; n++) {
                struct port *p = &d->ports[n];

                struct spa_io_buffers *io = p->io;

                if (io->buffer_id < p->n_buffers) {
                        port_reuse_buffer(p, io->buffer_id);
                        io->buffer_id = SPA_ID_INVALID;
                }

                if (spa_list_is_empty(&p->empty)) {
                        pw_log_error("livemix %p: out of buffers", d);
                        return -EPIPE;
                }

                b = spa_list_first(&p->empty, struct buffer, link);
                spa_list_remove(&b->link);
        
                od = b->buffer->datas;

                maxsize = od[0].maxsize;
        
                if (true) {
                        port_reuse_buffer(p, b->id);
                        return SPA_STATUS_OK;
                }

                od[0].chunk->offset = 0;
                od[0].chunk->size = maxsize;
                od[0].chunk->stride = 0;
        
                io->buffer_id = b->id;
                io->status = SPA_STATUS_HAVE_DATA;
        
                port_update_volume(p);
        }

        return SPA_STATUS_HAVE_DATA;
}

static void proxy_removed(void *object) {
        struct data *data = object;
	pw_log_info("%p: proxy removed", data);
}

static void proxy_destroy(void *object) {
        struct data *data = object;
	pw_log_info("%p: proxy destroy", data);
}

static void proxy_error(void *object, int seq, int res, const char *message) {
        struct data *data = object;
	pw_log_info("%p: proxy error: %s", data, message);
}

static void proxy_bound_props(void *data, uint32_t global_id, const struct spa_dict *props) {
}

static void node_event_destroy(void *data) {
}

static void node_event_info(void *object, const struct pw_node_info *info) {
        struct data *d = object;
        pw_log_info("%p: node_event_info", d);
}

static void node_state_changed(void *object, enum pw_node_state old,
		enum pw_node_state state, const char *error)
{
        struct data *d = object;
        pw_log_info("%p: node_state_changed: state:%d, error:%s", d, state, error);

	switch (state) {
	case PW_NODE_STATE_RUNNING:
		break;
	case PW_NODE_STATE_ERROR:
		break;
	default:
		break;
	}
}

static void node_drained(void *data) {
}

static const struct spa_node_methods impl_node = {
        SPA_VERSION_NODE_METHODS,
        .add_listener = impl_add_listener,
        .set_callbacks = impl_set_callbacks,
	.enum_params = impl_enum_params,
	.set_param = impl_set_param,
        .set_io = impl_set_io,
        .send_command = impl_send_command,
        .port_enum_params = impl_port_enum_params,
        .port_set_param = impl_port_set_param,
        .port_use_buffers = impl_port_use_buffers,
        .port_set_io = impl_port_set_io,
        .port_reuse_buffer = impl_port_reuse_buffer,
        .process = impl_node_process,
};

static const struct pw_proxy_events proxy_events = {
	PW_VERSION_PROXY_EVENTS,
	.removed = proxy_removed,
	.destroy = proxy_destroy,
	.error = proxy_error,
	.bound_props = proxy_bound_props,
};

static const struct pw_impl_node_events node_events = {
	PW_VERSION_IMPL_NODE_EVENTS,
	.destroy = node_event_destroy,
	.info_changed = node_event_info,
	.state_changed = node_state_changed,
};

static const struct pw_impl_node_rt_events node_rt_events = {
	PW_VERSION_IMPL_NODE_RT_EVENTS,
	.drained = node_drained,
};

static int make_node(struct data *data)
{
        struct pw_properties *props;
        struct pw_impl_factory *factory;

        if (false) {
                props = pw_properties_copy(data->props);

                factory = pw_context_find_factory(data->context, "adapter");

                if (factory == NULL) {
                        pw_log_error("%p: no adapter factory found", data);
                        return -ENOENT;
                }

                pw_properties_setf(props, "adapt.follower.node", "pointer:%p", data->node);
                pw_properties_set(props, "object.register", "false");

                data->adapter_node = pw_impl_factory_create_object(factory, NULL, PW_TYPE_INTERFACE_Node, PW_VERSION_NODE, props, 0);

                if (data->adapter_node == NULL) {
                        return -errno;
                }
                
                data->proxy = pw_core_export(data->core, PW_TYPE_INTERFACE_Node, NULL, data->adapter_node, 0);

                if (data->proxy == NULL) {
                        return -errno;
                }

                pw_impl_node_add_listener(data->adapter_node, &data->node_listener, &node_events, data);
                pw_impl_node_add_rt_listener(data->adapter_node, &data->node_rt_listener, &node_rt_events, data);
        } else {
                data->impl_node.iface = SPA_INTERFACE_INIT(SPA_TYPE_INTERFACE_Node, SPA_VERSION_NODE, &impl_node, data);
                data->node = pw_context_create_node(data->context, data->props, 0);

                if (data->node == NULL) {
                        return -errno;
                }

                pw_impl_node_set_implementation(data->node, &data->impl_node);
                data->proxy = pw_core_export(data->core, PW_TYPE_INTERFACE_Node, &data->props->dict, data->node, 0); 
        }

        pw_proxy_add_listener(data->proxy, &data->proxy_listener, &proxy_events, data);
        return 0;
}

static void on_core_error(void *data, uint32_t id, int seq, int res, const char *message)
{
        struct data *d = data;
 
        pw_log_error("error id:%u seq:%d res:%d (%s): %s",
                        id, seq, res, spa_strerror(res), message);
 
        if (id == PW_ID_CORE)
                pw_main_loop_quit(d->loop);
}

static const struct pw_core_events core_events = {
        PW_VERSION_CORE_EVENTS,
        .error = on_core_error,
};
 
int main(int argc, char *argv[])
{
        struct data data = { 0 };
        int err;
        int n;

        pw_init(&argc, &argv);
 
        data.loop = pw_main_loop_new(NULL);
        data.context = pw_context_new(pw_main_loop_get_loop(data.loop), NULL, 0);
        data.path = argc > 1 ? argv[1] : NULL;

	data.change_mask_all = SPA_NODE_CHANGE_MASK_FLAGS | SPA_NODE_CHANGE_MASK_PROPS | SPA_NODE_CHANGE_MASK_PARAMS;

        data.props = pw_properties_new(
                PW_KEY_NODE_AUTOCONNECT, "true",
                PW_KEY_NODE_EXCLUSIVE, "true",
                PW_KEY_NODE_NAME, "livemix",
                PW_KEY_MEDIA_NAME, "lmao",
                PW_KEY_MEDIA_TYPE, "Audio",
                PW_KEY_MEDIA_CATEGORY, "Playback",
                PW_KEY_MEDIA_ROLE, "Music",
                NULL
        );

        if (data.path) {
                pw_properties_set(data.props, PW_KEY_TARGET_OBJECT, data.path);
        }

	data.info = SPA_NODE_INFO_INIT();
        data.info.max_input_ports = 0;
        data.info.max_output_ports = 2;

	data.params[NODE_PropInfo] = SPA_PARAM_INFO(SPA_PARAM_PropInfo, 0);
	data.params[NODE_Props] = SPA_PARAM_INFO(SPA_PARAM_Props, SPA_PARAM_INFO_WRITE);
	data.params[NODE_EnumFormat] = SPA_PARAM_INFO(SPA_PARAM_EnumFormat, SPA_PARAM_INFO_READ);
	data.params[NODE_Format] = SPA_PARAM_INFO(SPA_PARAM_Format, SPA_PARAM_INFO_WRITE);
        data.info.props = &data.props->dict;
	data.info.params = data.params;
	data.info.n_params = N_NODE_PARAMS;
	data.info.change_mask = data.change_mask_all;
 
        data.port_change_mask_all = SPA_PORT_CHANGE_MASK_FLAGS | SPA_PORT_CHANGE_MASK_PROPS | SPA_PORT_CHANGE_MASK_PARAMS;

        char name[32];

        for (n = 0; n < MAX_PORTS; n++) {
                struct port *port = &data.ports[n];

                snprintf(name, sizeof(name), "out_%d", n);

                port->port_props = pw_properties_new(
                        PW_KEY_FORMAT_DSP, "32 bit float mono audio",
                        PW_KEY_PORT_NAME, name,
                        NULL
                );

                port->port_params[PORT_EnumFormat] = SPA_PARAM_INFO(SPA_PARAM_EnumFormat, SPA_PARAM_INFO_READ);
                port->port_params[PORT_Meta] = SPA_PARAM_INFO(SPA_PARAM_Meta, SPA_PARAM_INFO_READ);
                port->port_params[PORT_IO] = SPA_PARAM_INFO(SPA_PARAM_IO, SPA_PARAM_INFO_READ);
                port->port_params[PORT_Format] = SPA_PARAM_INFO(SPA_PARAM_Format, SPA_PARAM_INFO_WRITE);
                port->port_params[PORT_Buffers] = SPA_PARAM_INFO(SPA_PARAM_Buffers, 0);

                port->port_info = SPA_PORT_INFO_INIT();
                port->port_info.flags = 0;
                port->port_info.props = &port->port_props->dict;
                port->port_info.params = port->port_params;
                port->port_info.n_params = N_PORT_PARAMS;
                port->port_info.change_mask = data.port_change_mask_all;
                spa_zero(port->format);
                spa_list_init(&port->empty);
        }

        data.n_ports = MAX_PORTS;
        spa_hook_list_init(&data.hooks);
 
        if ((data.core = pw_context_connect(data.context, NULL, 0)) == NULL) {
                printf("can't connect: %m\n");
                return -1;
        }
 
        pw_core_add_listener(data.core, &data.core_listener, &core_events, &data);
 
        if ((err = make_node(&data)) != 0) {
                return err;
        }

        pw_main_loop_run(data.loop);
 
        pw_context_destroy(data.context);
        pw_main_loop_destroy(data.loop);
 
        return 0;
}