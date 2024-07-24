run-example:
    HOST=localhost PORT=3143 \
    USER=haisham PASS=password cargo run --example imap-listen

greenmail:
    docker run -t -i -p 3025:3025 -p 3110:3110 -p 3143:3143 -p 3465:3465 -p 3993:3993 -p 3995:3995 -p 8080:8080 greenmail/standalone:2.1.0-rc-1

send-mail:
    curl smtp://localhost:3025 --mail-from haisham@mail.com \
    --mail-rcpt haisham@mail.com --upload-file email.txt
