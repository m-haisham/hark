run-example:
    HOST=localhost PORT=3993 \
    USER=haisham PASS=password cargo run --example imap-listen

send-mail:
    curl smtp://localhost:3025 --mail-from username --mail-rcpt username --upload-file email.txt

up:
    docker-compose up -d

down:
    docker-compose down
