
#ifndef LIVEMIX_STREAM_PRIVATE_H
#define LIVEMIX_STREAM_PRIVATE_H


#define lm_stream_emit(s,m,v,...) spa_hook_list_call(&s->listener_list, struct lm_stream_events, m, v, ##__VA_ARGS__)
#define lm_stream_emit_destroy(s)		lm_stream_emit(s, destroy, 0)
#define lm_stream_emit_state_changed(s,o,n,e)	lm_stream_emit(s, state_changed,0,o,n,e)
#define lm_stream_emit_io_changed(s,i,a,t)	lm_stream_emit(s, io_changed,0,i,a,t)
#define lm_stream_emit_param_changed(s,i,p)	lm_stream_emit(s, param_changed,0,i,p)
#define lm_stream_emit_add_buffer(s,b)		lm_stream_emit(s, add_buffer, 0, b)
#define lm_stream_emit_remove_buffer(s,b)	lm_stream_emit(s, remove_buffer, 0, b)
#define lm_stream_emit_process(s)		lm_stream_emit(s, process, 0)
#define lm_stream_emit_drained(s)		lm_stream_emit(s, drained,0)
#define lm_stream_emit_control_info(s,i,c)	lm_stream_emit(s, control_info, 0, i, c)
#define lm_stream_emit_command(s,c)		lm_stream_emit(s, command,1,c)
#define lm_stream_emit_trigger_done(s)		lm_stream_emit(s, trigger_done,2)


struct lm_stream {
	struct pw_core *core;			/**< the owner core */
	struct spa_hook core_listener;

	struct spa_list link;			/**< link in the core */

	char *name;				/**< the name of the stream */
	struct pw_properties *properties;	/**< properties of the stream */

	uint32_t node_id;			/**< node id for remote node, available from
						  *  CONFIGURE state and higher */
	enum lm_stream_state state;		/**< stream state */
	char *error;				/**< error reason when state is in error */
	int error_res;				/**< error code when in error */

	struct spa_hook_list listener_list;

	struct pw_proxy *proxy;
	struct spa_hook proxy_listener;

	struct pw_impl_node *node;
	struct spa_hook node_listener;
	struct spa_hook node_rt_listener;

	struct spa_list controls;

	unsigned int driving: 1;
	unsigned int sc_pagesize;
};

int pw_impl_node_trigger(struct pw_impl_node *node);

#define PW_LOG_OBJECT_POD	(1<<0)
#define PW_LOG_OBJECT_FORMAT	(1<<1)
void pw_log_log_object(enum spa_log_level level, const struct spa_log_topic *topic,
		const char *file, int line, const char *func, uint32_t flags,
		const void *object);

#define pw_log_object(lev,t,fl,obj)				\
({								\
	if (SPA_UNLIKELY(pw_log_topic_enabled(lev,t)))		\
		pw_log_log_object(lev,t,__FILE__,__LINE__,	\
				__func__,(fl),(obj));		\
})

#define pw_log_pod(lev,pod) pw_log_object(lev,PW_LOG_TOPIC_DEFAULT,PW_LOG_OBJECT_POD,pod)
#define pw_log_format(lev,pod) pw_log_object(lev,PW_LOG_TOPIC_DEFAULT,PW_LOG_OBJECT_FORMAT,pod)

int pw_loop_check(struct pw_loop *loop);

#define ensure_loop(loop,...) ({							\
	int res = pw_loop_check(loop);							\
	if (res != 1) {									\
		pw_log_warn("%s called from wrong context, check thread and locking: %s",	\
				__func__, res < 0 ? spa_strerror(res) : "Not in loop");	\
		fprintf(stderr, "*** %s called from wrong context, check thread and locking: %s\n",\
				__func__, res < 0 ? spa_strerror(res) : "Not in loop");	\
		/* __VA_ARGS__ */							\
	}										\
})

#endif /* LIVEMIX_STREAM_PRIVATE_H */