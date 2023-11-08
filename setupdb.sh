#!/usr/bin/env sh
SCRIPT_DIR=$( cd -- "$( dirname -- "$0" )" &> /dev/null && pwd )

success() {
    echo "Development DB ready to use."
    echo
    echo "   Update the database schema by running:"
    echo "   $ cargo sqlx migrate run"
    echo
    echo "   Stop container:                 Remove container:"
    echo "   $ docker stop zling-db         $ docker rm zling-db"
    echo
    echo "   Get an SQL shell inside the database:"
    echo "   $ docker exec -it zling-db psql -U zling-backend"
    echo
    echo "   Happy hacking!"
    echo
    exit 0
}

post_container_setup() {
    if [ -z "$SKIP_MIGRATIONS" ] || [ "$SKIP_MIGRATIONS" = 0 ]; then
        echo "Checking and applying any pending migrations..."
        cargo sqlx migrate run

        if [ $? -eq 0 ]; then
            success
        else
            echo "error: Failed to apply migrations!"
            exit 1
        fi
    fi

    success
}

if ([ -z "$SKIP_MIGRATIONS" ] || [ "$SKIP_MIGRATIONS" = 0 ]) && ([ -z "$SKIP_TOOL_CHECKS" ] || [ "$SKIP_TOOL_CHECKS" = 0 ]) ; then
    # check if sqlx is installed in ~/.cargo/bin/sqlx
    if [ ! -f ~/.cargo/bin/sqlx ]; then
        echo "error: sqlx-cli not found in ~/.cargo/bin/sqlx (needed to run migrations)"

        # prompt user whether to install sqlx
        printf "Install sqlx-cli with cargo install? [y/n]: "
        read answer
        echo
        if [ "$answer" = "${answer#[Yy]}" ]; then
            # user said no (yes the above if statement is weird but trust me
            # it's not a typo)
            echo "error: please install sqlx-cli in some way before running this script"
            echo "see: https://github.com/launchbadge/sqlx/tree/main/sqlx-cli#install"
            exit 1
        fi

        echo "[$] cargo install sqlx-cli --no-default-features --features native-tls,postgres"
        # do the install
        cargo install sqlx-cli --no-default-features --features native-tls,postgres
    fi
fi


# check if docker container with name zling-db already exists
if [ "$(docker ps -aq -f name=zling-db)" ]; then
    # check if container is running
    if [ "$(docker ps -aq -f status=running -f name=zling-db)" ]; then
        post_container_setup
    fi

    echo "Starting container..."
    docker start zling-db >/dev/null
    echo "Waiting 3 seconds for the DB to start..."
    sleep 3

    # ensure exit code is 0
    if [ $? -eq 0 ]; then
        post_container_setup
    else
        echo "error: Failed to start container!"
        exit 1
    fi
fi

echo "Creating database container..."
docker run -d -e POSTGRES_USER=zling-backend -e POSTGRES_PASSWORD=dev -p 127.0.0.1:5432:5432 --name zling-db postgres || exit 1
echo "Waiting 3 seconds for the DB to start..."
sleep 3
post_container_setup