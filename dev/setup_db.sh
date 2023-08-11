#!/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# check if docker container with name zling-db already exists
if [ "$(docker ps -aq -f name=zling-db)" ]; then
    echo "A container with name zling-db already exists."
    # check if container is running
    if [ "$(docker ps -aq -f status=running -f name=zling-db)" ]; then
        echo "Nothing to do. The container is already running."
        exit 0
    fi
    echo "Starting container..."
    docker start zling-db >/dev/null

    # ensure exit code is 0
    if [ $? -eq 0 ]; then
        echo "Container started. Database is up."
    else
        echo "error: Failed to start container!"
        exit 1
    fi    

    exit 0
fi

echo "Creating database container..."
docker run -d -e POSTGRES_USER=zling-backend -e POSTGRES_PASSWORD=dev -p 127.0.0.1:5432:5432 --name zling-db postgres || exit 1
echo "Waiting 3 seconds for the db to start..."
sleep 3
echo "Creating schema..."
docker exec -i zling-db psql -U zling-backend -h 127.0.0.1 zling-backend < $SCRIPT_DIR/../sql/create-tables.sql || exit 1
echo
echo "Database is now running!"
echo
docker ps -f name=zling-db
echo
echo "   To stop the database container, run:"
echo "   $ docker stop zling-db"
echo "   And to remove it, run:"
echo "   $ docker rm zling-db"
echo
echo "   To start the database container if it's stopped,"
echo "   $ docker start zling-db"
echo
echo "   To get an SQL shell inside the database container, run:"
echo "   $ docker exec -it zling-db psql -U zling-backend"
echo
echo "   To run an SQL file inside the database container:"
echo "   $ docker exec -i zling-db psql -U zling-backend < your-file.sql"
echo
echo "   Happy hacking!"
echo