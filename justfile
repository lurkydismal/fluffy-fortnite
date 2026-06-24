#!/usr/bin/env -S just --justfile

logs_dir := "{{ logs_dir }}"
tmux_session := "quick"
tmux := require("tmux")

# Redirects to debug build to avoid accidental optimized builds during development.
default: run-debug

# Builds the project in debug mode.
build-debug:
    dotnet build -c Debug

# Start the selected development service.
[arg('type', pattern='all|server|client')]
run-debug type='all': build-debug
    just run {{ type }}

# Builds the project in release mode.
build-release:
    dotnet build -c Release

# Start the selected production service.
[arg('type', pattern='all|server|client')]
run-release type='all': build-release
    just run {{ type }}

# Build all projects in the Tools configuration
build-tools:
    dotnet build -c Tools

# Start server/client separately or together in a tmux session.
[arg('type', pattern='all|server|client')]
run type='all':
    #!/usr/bin/env bash
    set -euo pipefail
    start_server() {
        dotnet run --project Content.Goobstation.Server --no-build
    }
    start_client() {
        dotnet run --project Content.Goobstation.Client --no-build
    }
    ensure_tmux_session() {
        if {{ tmux }} has-session -t '{{ tmux_session }}' 2>/dev/null; then # Check if session already exists
            :                                               # Do nothing if session exists
        else                                                # If session does not exist
            {{ tmux }} new-session -d -s '{{ tmux_session }}' -n '{{ tmux_session }}' # Create detached tmux session and initial window

            {{ tmux }} split-window -h -t '{{ tmux_session }}':0 # Split first window horizontally into two panes

            {{ tmux }} send-keys -t '{{ tmux_session }}':0.0 'just run server' C-m # Start server script in left pane
            {{ tmux }} send-keys -t '{{ tmux_session }}':0.1 'just run client' C-m # Start client script in right pane

            {{ tmux }} set-option -p -t '{{ tmux_session }}':0.0 @title "Server" # Set custom pane metadata title for left pane
            {{ tmux }} set-option -p -t '{{ tmux_session }}':0.1 @title "Client" # Set custom pane metadata title for right pane

            {{ tmux }} set-option -t '{{ tmux_session }}' pane-border-status top         # Enable pane border status display
            {{ tmux }} set-option -t '{{ tmux_session }}' pane-border-format "#{@title}" # Display custom @title in pane border

            {{ tmux }} rename-window -t '{{ tmux_session }}':0 "Server | Client # (Ctrl-b: ←/→ switch | d detach)" # Set window title with hint
        fi
    }
    attach_tmux_session() {
        if [ -n "${TMUX-}" ]; then              # Check if already inside a tmux session
            {{ tmux }} switch-client -t '{{ tmux_session }}' # Switch current client to target session
        else                                 # If not inside tmux
            {{ tmux }} attach -t '{{ tmux_session }}'        # Attach to session in new tmux client
        fi

        {{ tmux }} set-option -t '{{ tmux_session }}':0 mouse on # Enable tmux mouse support for window 0 only
    }
    case '{{ type }}' in
        client) start_client ;;
        server) start_server ;;
        all)
            ensure_tmux_session
            attach_tmux_session
            ;;
    esac

# Execute tests with code coverage and write results to log files
[arg('type', pattern='all|unit|integration|yaml')]
run-tests type='all':
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p {{ logs_dir }}
    run() {
        log_file=$1
        shift
        rm -f "{{ logs_dir }}/$log_file"
        "$@" > "{{ logs_dir }}/$log_file" 2>&1
    }
    run_unit() {
        run Content.Tests.log \
            dotnet test \
            --collect:"XPlat Code Coverage" \
            Content.Tests/Content.Tests.csproj \
            -c DebugOpt \
            -- \
            NUnit.ConsoleOut=0
    }
    run_integration() {
        run Content.IntegrationTests.log \
            dotnet test \
            --collect:"XPlat Code Coverage" \
            Content.IntegrationTests/Content.IntegrationTests.csproj \
            -c DebugOpt \
            -- \
            NUnit.ConsoleOut=0 \
            NUnit.MapWarningTo=Failed
    }
    run_yaml() {
        run Content.YAMLLinter.log \
            dotnet run \
            --project Content.YAMLLinter/Content.YAMLLinter.csproj \
            -c DebugOpt \
            -- \
            NUnit.ConsoleOut=0
    }
    case '{{ type }}' in
        unit) run_unit ;;
        integration) run_integration ;;
        yaml) run_yaml ;;
        all)
            run_unit
            run_integration
            run_yaml
            ;;
    esac

# Pull images for all services, skipping services that have no build context.
docker-pull images='':
    if [ -n "{{ images }}" ]; then \
        for image in {{ images }}; do \
            docker pull "$image"; \
        done; \
    fi
    docker compose  pull --ignore-buildable

# Start all services.
docker-up-all:
    docker compose up -d

# Start one service in detached mode.
docker-up image='postgres':
    docker compose up -d {{ image }}

# Stop all running containers in the current Docker Compose project without removing containers, networks, or volumes.
docker-stop:
    docker compose stop

# Stop and remove containers, networks, default resources, and optionally volumes created by the current Docker Compose project.
[arg('remove_volumes', pattern='false|true')]
docker-down remove_volumes='false':
    if [ "{{ remove_volumes }}" = "true" ]; then \
        docker compose down --volumes; \
    else \
        docker compose down; \
    fi

# Open an interactive shell inside a running service container.
docker-interact image='postgres':
    docker compose exec '{{ image }}' bash

# Stream logs from all services and follow output
docker-logs:
    docker compose logs -f

# Attach to a running service container without signal proxying.
docker-attach image='postgres':
    -docker compose attach --sig-proxy=false '{{ image }}' sh

# Show a one-time snapshot of resource usage statistics for containers in the current Docker Compose project.
docker-stats:
    docker compose stats --no-stream

# Remove dangling Docker images that are no longer referenced by any tag.
docker-remove-unused-images:
    docker images -f "dangling=true" -q | xargs -r docker rmi

# Generate a cryptographically secure random alphanumeric token of length `N`. Uses `openssl rand` as the entropy source, encodes as Base64, removes padding and non-alphanumeric output, then retries until the result is exactly `N` characters using only `[A-Za-z0-9]`.
generate-token length='32':
    #!/usr/bin/env bash
    set -euo pipefail
    while true; do
    s=$(openssl rand -base64 "$(({{ length }} + 3))" | tr -d '\n')
    s="${s%%=*}"   # remove all trailing '='
    s="${s:0:{{ length }}}"   # cut back to requested length
    # allow only base64 alnum
    if [[ "$s" =~ ^[A-Za-z0-9]+$ && ${#s} -eq {{ length }} ]]; then
        echo "$s"
        break
    fi
    done
