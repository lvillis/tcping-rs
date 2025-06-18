<!-- ─── Language Switch & ToC (top-right) ─────────────────────────── -->
<div align="right">

<a href="README.md">🇺🇸 English</a> ·
<span style="color:#999;">🇨🇳 中文</span> &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;|&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; 目录↗️

</div>

<h1 align="center"><code>tcping-rs</code></h1>

<p align=center>🛠️ tcping-rs: 检测TCP端口连通性和延迟的工具。</p>

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/tcping.svg)](https://crates.io/crates/tcping)&nbsp;
[![Repo Size](https://img.shields.io/github/repo-size/lvillis/tcping-rs?color=328657)](https://github.com/lvillis/tcping-rs)&nbsp;
[![CI](https://github.com/lvillis/tcping-rs/actions/workflows/ci.yaml/badge.svg)](https://github.com/lvillis/tcping-rs/actions)&nbsp;
[![Docker Pulls](https://img.shields.io/docker/pulls/lvillis/tcping?style=flat-square)](https://hub.docker.com/r/lvillis/tcping)&nbsp;
[![Image Size](https://img.shields.io/docker/image-size/lvillis/tcping/latest?style=flat-square)](https://hub.docker.com/r/lvillis/tcping)&nbsp;
[![Say Thanks](https://img.shields.io/badge/Say%20Thanks-!-1EAEDB.svg)](mailto:lvillis@outlook.com?subject=Thanks%20for%20tcping-rs!)

</div>

---

## ✨ 优势

| 亮点               | 描述                                                                |
|--------------------|---------------------------------------------------------------------|
| **纯Rust实现**     | 无需运行时依赖的极小静态程序                                        |
| **免ICMP协议**     | 在传统`ping`被屏蔽的场景仍可用，仅依靠TCP握手检测                   |
| **跨平台**         | 支持Linux、macOS、Windows、*BSD及所有Rust T1级支持平台              |
| **多种模式**       | `-t` 持续运行, `-c` 指定次数, `-e` 支持提前退出                     |
| **机器可读输出**   | `-o` 支持JSON / CSV格式, 适用于脚本处理与监控                       |
| **抖动统计**       | `-j` 显示延迟波动情况(包含P95百分位)                                |
| **Docker镜像**     | 提供多架构镜像(`amd64` / `arm64`)，适配CI/CD流水线与Kubernetes任务  |


## 用法

```bash
tcping <host:port> [-c count] [-t] [-e] [-j] [-o mode]
```

参数:

- `host:port` 要检测的主机和端口
- `-c count` 指定检测次数(默认: 4)
- `-t` 开启持续检测
- `-e` 目标机器握手成功后立即退出
- `-j` 计算和显示抖动统计
- `-o mode` 设置输出格式 (`normal`, `json`, `csv`)
- `-h` 打印帮助信息
- `-V` 打印程序版本

## 示例

```bash
$ tcping github.com:443

Resolved address in 0.9340 ms
Probing 140.82.113.4:443/tcp - Port is open - time=12.7510ms
Probing 140.82.113.4:443/tcp - Port is open - time=12.4270ms
Probing 140.82.113.4:443/tcp - Port is open - time=11.4410ms
Probing 140.82.113.4:443/tcp - Port is open - time=12.7510ms

--- 140.82.113.4:443 tcping statistics ---
4 probes sent, 4 successful, 0.00% packet loss
Round-trip min/avg/max = 11.4410ms/12.3425ms/12.7510ms
Address resolved in 0.9340 ms
```

## 安装指南

### 下载发行版

从[版本发布页](https://github.com/lvillis/tcping-rs/releases)获取预编译的二进制文件

* 访问[版本发布页](https://github.com/lvillis/tcping-rs/releases)。
* 下载适用于您操作系统的对应二进制文件。
* 解压可执行文件，将其放置在系统PATH环境变量包含的目录中。

### 使用Docker

使用Docker镜像运行`tcping-rs`:

```shell
docker run --rm docker.io/lvillis/tcping:latest <host:port> [options]

```

## 关于

该工具通过TCP协议测量服务器延迟。采用Rust语言开发，并集成clap库实现命令行参数解析。
