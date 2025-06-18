# Deadlock API Match Data Ingestor

This project allows you to ingest match data to `deadlock-api.com`. It fetches match metadata and sends it to the Deadlock API.

## Disclaimer

This project is not affiliated with Valve Corporation or Steam. It is a third-party project that uses the Steam API to fetch match data.
Calling the Steam API requires a Steam account with Deadlock Game access.
We take no responsibility for any misuse of this project, account bans, or any other issues that may arise from using this project.

## How it works

1. We create a connection with your steam account to steam servers (locally from your PC).
2. We send a request to steam to fetch the salts for the matches you want to fetch metadata for (locally from your PC).
3. We then send the salts to the Deadlock API.

So the only thing that is sent to deadlock-api.com are the match salts, **no credentials are sent to the API**.

## Usage

### Using Docker

You can run the script using Docker:

```bash
docker run --rm -it --pull always ghcr.io/deadlock-api/public-ingest:latest public-ingest \
  --username STEAM_USERNAME --password STEAM_PASSWORD --match_ids 34044166 34044167 # ...
```

### Building Locally

To build and run the code locally, you need to have Rust and Cargo installed.

1. Install protobuf compiler

    ```sh
    apt install protobuf-compiler
    ```

2. Clone the repository:

    ```sh
    git clone https://github.com/deadlock-api/public-ingest.git
    cd public-ingest
    ```

3. Build the project:

    ```sh
    cargo build --release
    ```

4. Run the script:

    ```sh
    cargo run --release -- --username STEAM_USERNAME --password STEAM_PASSWORD --match_ids 34044166 34044167 # ...
    ```

## Configuration

The script requires the following arguments:

- `--username`: Your Steam username. You can also use an environment variable `STEAM_USERNAME`.
- `--password`: Your Steam password. You can also use an environment variable `STEAM_PASSWORD`.
- `--match_ids`: A list of match IDs to fetch and ingest.
