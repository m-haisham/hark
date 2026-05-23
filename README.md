# Hark

Hark is a IMAP to http proxy server. It allows you to listen for new emails on an IMAP server and forward them to a specified HTTP endpoint. This can be useful for integrating email notifications into your applications or services.

This is intended to be used as a sidecar service that runs alongside your main application. It is designed to be stateless, so it can be easily restarted or scaled without losing any data. This is useful because you dont want the imap connection to be interrupted when you deploy a new version of your main application.

This project is still in development, but the basic functionality is implemented and working. But I suspect there are some edge cases that I haven't tested yet, so use it with caution.

## Features

- Connect to an IMAP server and listen for new emails
- Forward new email notifications to a specified HTTP endpoint
- Configurable authentication via username/password or OAuth2
- Support for multiple IMAP accounts and endpoints
- Automatic reconnection for OAuth2 tokens when they expire

## Usage

There are a few ways to use Hark. I recommend using the Docker image for ease of use, but you can also run it directly from the source code.

### Docker

You can run Hark using Docker with the following command:

```bash
docker run -d \
  -p 3000:3000 \
  -v /path/to/config.toml:/app/config.toml \
  hshm/hark:latest
```

### Source Code

To run Hark from the source code, follow these steps:

1. Clone the repository:

    ```bash
    git clone https://github.com/haisham/hark.git
    cd hark
    ```

2. Build frontend:

    ```bash
    npm install
    npm run build
    ```

3. Run the server:

    ```bash
    cargo run --release
    ```

## Configuration

Hark can be configured using a `config.toml` file. Here is an example configuration:

```toml
[server]
host = "localhost"
port = 3000

[connections.default]
host = "localhost"
port = 3143
auth = "password"
tls = false
username = "username"
password = "password"
mailbox = "INBOX"

[anchor]
fetch_url = "http://localhost:4000/fetch"
callback_url = "http://localhost:4000/callback"
ping = false

[anchor.headers]
Authorization = "Bearer 123"
```

The server if configured will call the specified HTTP fetch endpoint, and use the data returned to instantiate the connections.
Since hark is stateless, this can be used to persist which connections you want to listen to. For example, you can store your accounts
in a database that is connected to your main application, and have the fetch endpoint return the accounts you want to listen to. This way, the configurations with persist even if the server is restarted.

## Contributing

Contributions are welcome! If you have any ideas for new features or improvements, please open an issue or submit a pull request.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for more details.
