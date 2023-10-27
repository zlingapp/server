#!/usr/bin/env sh

./setupdb.sh

if [ $? -ne 0 ]; then
    echo "error: Failed to setup database, stopping build."
    exit 1
fi

IMAGE_NAME=$1

# if TAG is empty, then
if [ -z "$IMAGE_NAME" ]; then
    IMAGE_NAME="ghcr.io/zlingapp/server"
fi 

echo "Building image ${IMAGE_NAME}"
echo "*********************************"
docker build . -t $IMAGE_NAME --network host