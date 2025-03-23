# Deadlock API Match Data Ingestor

This project allows you to ingest match data to `deadlock-api.com`. It fetches match metadata and sends it to the Deadlock API.

## Usage

### Using Docker

You can run the script using Docker:

```bash
docker run --rm -it ghcr.io/deadlock-api/public-ingest:latest public-ingest \
  --username STEAM_USERNAME --password STEAM_PASSWORD --match_ids 34044166 34044167 # ...
```

### Building Locally

To build and run the code locally, you need to have Rust and Cargo installed.

1. Clone the repository:

    ```sh
    git clone https://github.com/deadlock-api/public-ingest.git
    cd public-ingest
    ```

2. Build the project:

    ```sh
    cargo build --release
    ```

3. Run the script:

    ```sh
    cargo run --release -- --username STEAM_USERNAME --password STEAM_PASSWORD --match_ids 34044166 34044167 # ...
    ```

## Configuration

The script requires the following arguments:

- `--username`: Your Steam username. You can also use an environment variable `STEAM_USERNAME`.
- `--password`: Your Steam password. You can also use an environment variable `STEAM_PASSWORD`.
- `--match_ids`: A list of match IDs to fetch and ingest.
