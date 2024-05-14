# servus

A dead simple application to serve files and directories over HTTP.

## Usage

### Quick Local File Sharing

You can simply use the application in your terminal to serve files in your local network, for example if you want to share files from one device to another. Thats particularly useful because almost every modern device supports download via HTTP.

```
$ servus --serve ./data
[2022-11-08T18:20:53Z INFO  servus] Bound to address 172.17.0.3:80
[2022-11-08T18:20:53Z INFO  servus] LocalStore: ./data -> /
```

Let's assume you have a file calles `big.file` in the `./data` directory. After starting servus, you can now download the file on another device by opening `http://172.17.0.3/big.file` in your browser.

### Content Delivery Server

You can - of course - also set up servus as a small content delivery serivce. Therefore, you can simply use the provided Docker image.

`/etc/servus/config.yml`
```yml
address: "0.0.0.0:8081"
stores:
  - type: "Local"
    directory: "testdata"
    servepath: "localdata"
    browse: true
  - type: "S3"
    servepath: "s3data"
    accesskey: minioadmin
    secretkey: minioadmin
    bucket: test
    endpoint: http://localhost:9000
    browse: false
```

```
$ docker run \
    --name servus \
    --volume /var/opt/data:/data \
    --volume /etc/servus/config.yml:/etc/servus/config.yml:ro \
    ghcr.io/zekrotja/servus:latest \
        --config /etc/servus/config.yml
```
