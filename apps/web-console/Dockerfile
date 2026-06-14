FROM rust:1.96-bookworm AS builder

WORKDIR /workspace
ENV CARGO_BUILD_JOBS=1
RUN rustup target add wasm32-unknown-unknown \
    && cargo install trunk --version 0.21.14 --locked --jobs 1
COPY apps/web-console ./apps/web-console
WORKDIR /workspace/apps/web-console
ENV CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_DEV_DEBUG=0 \
    CARGO_BUILD_JOBS=1
RUN NO_COLOR=false trunk build --release --locked --jobs 1

FROM nginx:1.27-alpine AS runtime

COPY --from=builder /workspace/apps/web-console/dist /usr/share/nginx/html
COPY apps/web-console/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 8081
CMD ["nginx", "-g", "daemon off;"]
