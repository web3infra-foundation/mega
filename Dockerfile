# Use a Rust base image
FROM rust:latest as builder

# Set the working directory
WORKDIR /usr/src/mega

# Copy the Rust project files to the working directory
COPY . .

# Build the Rust executable
RUN cargo build --release

# Create a new image without the build dependencies
FROM debian:bullseye-slim

# Set the working directory
WORKDIR /usr/src/mega

# Copy the built executable from the builder stage
COPY --from=builder /usr/src/mega/target/release/mega .

# Run the Rust executable command
CMD ["./mega", "https", "--host", "0.0.0.0", "-d", "postgres"]
