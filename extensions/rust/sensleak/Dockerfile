FROM c9-ssl32-gh19

ARG USERNAME="rust"
USER $USERNAME

# Create and set permissions for workdir directory
USER root
RUN mkdir -p /workdir && chown $USERNAME:$USERNAME /workdir
USER $USERNAME

WORKDIR /workdir

# Copy artifacts
COPY ./analysis_sensleak ./analysis_sensleak
COPY ./bin/scan ./bin/scan
COPY ./analyzers.json ./analyzers.json
COPY ./gitleaks.toml ./gitleaks.toml
COPY ./.env ./.env

# Required in .env
RUN mkdir -p /tmp/analysis-repos

ENV RUST_BACKTRACE=1
CMD ["./analysis_sensleak"]
