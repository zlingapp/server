#!/usr/bin/env sh

./dev/setup_db.sh

if [ $? -ne 0 ]; then
    echo "error: Failed to setup database, stopping build."
    exit 1
fi

IMAGE_NAME=$1

# if TAG is empty, then
if [ -z "$IMAGE_NAME" ]; then
    IMAGE_NAME="zling-server"
fi 

echo "Building image ${IMAGE_NAME}"
echo "*********************************"
docker build . -t $IMAGE_NAME --network host