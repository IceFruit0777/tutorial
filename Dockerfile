# 准备阶段
FROM lukemathwalker/cargo-chef:latest-rust-1.82.0 AS chef
# 工作目录切换到`app`(相当于`cd app`)
# `app`目录由docker自动创建
WORKDIR /app
RUN apt update && apt install lld clang -y

FROM chef AS planner
COPY . .
# 为项目计算出一个类似于锁的文件
RUN cargo chef prepare --recipe-path recipe.json

# 构建阶段
FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json
# 构建项目的依赖关系，而不是我们的程序
RUN cargo chef cook --release --recipe-path recipe.json
# 至此，如果依赖树不变，那么所有的分层都应该被缓存起来
COPY . .
# 开始构建二进制文件
# 使用release参数优化构建速度
ENV SQLX_OFFLINE=true
RUN cargo build --release

# 运行时阶段
FROM debian:bookworm-slim AS runtime

WORKDIR /app
# 安装 OpenSSL: 通过一些依赖动态链接
# 安装 ca-certificates: 在建立 HTTPS 连接时，需要验证 TLS 证书
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # 清理
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
# 从构建环境中复制已编译的二进制文件到运行时环境中
COPY --from=builder /app/target/release/tutorial tutorial
# 在运行时需要的配置文件
COPY config config
# 当执行`docker run`时，启动二进制文件
ENV APP_ENVIROMENT=production
ENTRYPOINT [ "./tutorial" ]
