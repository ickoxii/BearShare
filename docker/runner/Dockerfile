FROM debian:12

ENV DEBIAN_FRONTEND=noninteractive
ENV RUNNER_VERSION=2.330.0

# Base dependencies
RUN apt-get update \
    && apt-get install -y \
    ca-certificates \
    curl \
    gnupg \
    sudo \
    lsb-release

# Add Docker's official GPG key:
RUN install -m 0755 -d /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc \
    && chmod a+r /etc/apt/keyrings/docker.asc

# Add the repository to Apt sources:
RUN tee /etc/apt/sources.list.d/docker.sources <<EOF
Types: deb
URIs: https://download.docker.com/linux/debian
Suites: $(. /etc/os-release && echo "$VERSION_CODENAME")
Components: stable
Signed-By: /etc/apt/keyrings/docker.asc
EOF

RUN apt-get update

# Install Docker packages
RUN apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

RUN rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash runner \
    && echo "runner ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers \
    && usermod -aG docker runner

# Create runner user
USER runner
WORKDIR /home/runner

ENTRYPOINT ["tail", "-f", "/dev/null"]
