FROM redis:alpine

RUN mkdir -p /redis
WORKDIR /redis
COPY cluster.conf .
RUN chown redis:redis /redis/cluster.conf
EXPOSE 6379
COPY --chmod=755 cluster-entrypoint.sh .
ENTRYPOINT ["/redis/cluster-entrypoint.sh"]