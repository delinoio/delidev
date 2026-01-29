FROM node:24

# Enable corepack and install pnpm
RUN corepack enable && corepack prepare pnpm@latest --activate

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"
