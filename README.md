# BearShare

BearShare is a collaborative text editor that uses Replicated Growable Arrays (RGAs)[^1] to handle conflict-resolution and eventual consistency.

# Running the Project

Dependencies: `docker`, `python`

<details>
  <summary>Containerize the Server and Database</summary>

  1. Build the database first
     ```bash
     docker compose -f docker/local.docker-compose.yml up db -d --build
     ```

  1. Build the server
     ```bash
     docker compose -f docker/ci.docker-compose.yml up server -d --build
     ```

  1. Run the frontend
     ```bash
     cd frontend
     python -m http.server 8000
     ```

     Navigate to `http://localhost:8080`

  1. Cleaning up
     ```bash
     docker compose -f docker/ci.docker-compose.yml down server -v
     docker compose -f docker/local.docker-compose.yml down db -v
     ```
</details>

# References

[^1]: [Replicated abstract data types: Building blocks for collaborative applications (Roh et al., 2011)](http://csl.snu.ac.kr/papers/jpdc11.pdf)
