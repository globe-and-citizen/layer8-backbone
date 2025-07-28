
### Run `./docker_run.sh init` to start the services and upload the certificate
### Run `./docker_run.sh start` to start the services
### Run `./docker_run.sh upload` to upload the certificate
### Run `./docker_run.sh clean` to delete mounted data

upload_cert() {
  # login to get token
  TOKEN=$(curl --silent --location 'http://localhost:5001/api/v1/login-client' \
    --header 'Content-Type: application/json' \
    --data '{
      "password": "12341234",
      "username": "layer8"
  }' | jq -r '.token')

  echo "Token is: $TOKEN"

  RP_CERT=$(awk '{printf "%s\\n", $0}' "./reverse-proxy/ntor_cert.pem")
  echo "File content: $RP_CERT"

  curl --location 'http://localhost:5001/api/upload-certificate' \
    --header 'Content-Type: application/json' \
    --header "Authorization: Bearer $TOKEN" \
    --data "{
      \"certificate\": \"$RP_CERT\"
    }"
}

start() {
  mkdir logs
  touch logs/reverse-proxy.log
  touch logs/forward-proxy.log
  docker compose up -d
}

clean() {
  docker compose down
  rm -rf logs
  rm -rf layer8-volumes/influxdb2-data
  rm -rf layer8-volumes/pg-data
}

# main
if [ -z "$1" ]; then
  echo "Missing argument"
  exit 1
fi

if [ "$1" = "init" ]; then
  start
  sleep 30
  upload_cert
elif [ "$1" = "start" ]; then
  start
elif [ "$1" = "upload" ]; then
  upload_cert
elif [ "$1" = "clean" ]; then
  clean
fi


