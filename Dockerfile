FROM dhi.io/dotnet:9-sdk-alpine AS base
RUN apk add --no-cache bash
WORKDIR /app

FROM base AS dev
RUN apk add --no-cache git && \
    git config --global --add safe.directory /app

FROM dev AS dev-all
CMD [ "/bin/bash", "-lc", "Scripts/sh/buildAllDebug.sh && Scripts/sh/runQuickAll.sh" ]

FROM dev AS dev-server
CMD [ "/bin/bash", "-lc", "Scripts/sh/buildAllDebug.sh && Scripts/sh/runQuickServer.sh" ]

FROM dev AS dev-client
CMD [ "/bin/bash", "-lc", "Scripts/sh/buildAllDebug.sh && Scripts/sh/runQuickClient.sh" ]

# Production stage
FROM base AS prod
COPY . .
RUN bash Scripts/sh/buildAllRelease.sh

FROM dhi.io/dotnet:9-alpine AS prod-runtime
RUN apk add --no-cache bash
COPY --from=prod /app/bin /app/bin
COPY --from=prod /app/Scripts/sh /app/Scripts/sh

FROM prod-runtime AS prod-all
RUN apk add --no-cache tmux
CMD [ "/bin/bash", "-lc", "Scripts/sh/runQuickAll.sh" ]

FROM prod-runtime AS prod-server
CMD [ "/bin/bash", "-lc", "Scripts/sh/runQuickServer.sh" ]

FROM prod-runtime AS prod-client
CMD [ "/bin/bash", "-lc", "Scripts/sh/runQuickClient.sh" ]
