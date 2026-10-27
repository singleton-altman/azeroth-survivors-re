# Docker 本地部署说明(azeroth-survivors)

本目录在上游仓库基础上追加了 Docker 部署三件套(不改上游任何文件):

| 文件 | 作用 |
|---|---|
| `Dockerfile` | 多阶段构建:rust:1.98.1 编译 wasm 客户端 + wenilla-host → debian:bookworm-slim 运行时(镜像仅 ~330MB) |
| `docker-compose.yml` | 一键起服务:端口 `127.0.0.1:8090`,游戏数据只读挂载 |
| `fetch-data-v4.sh` | IPv4 强制 + 24 并发分段 + 卡死自愈重试的游戏数据下载器(替代 `scripts/fetch-data.sh`) |

## 首次部署

```bash
# 1. 下载游戏数据(~5.2GB,来自 archive.org;已完成分片自动跳过,可随时中断重跑)
bash fetch-data-v4.sh

# 2. 构建镜像(工具链/依赖全部走国内镜像,详见 Dockerfile 内注释)
docker compose build

# 3. 启动(wenilla-host 启动时会打开 MPQ 链,数据必须在位)
docker compose up -d

# 4. 验证
curl -sI http://127.0.0.1:8090/ | head -3
```

浏览器打开 **http://127.0.0.1:8090/** 即可游玩。**需要支持 WebGPU 的浏览器**(Chrome/Edge 113+;Safari 26+),否则只能看到 glue 界面——那是默认 WebGPU 后端的限制,如需 WebGL2 回退:`docker compose build --build-arg WEB_BACKEND=webgl2`。

## 日常使用

```bash
docker compose up -d      # 启动
docker compose logs -f    # 看日志
docker compose down       # 停止
```

也可以不用 compose:

```bash
docker run --rm -p 127.0.0.1:8090:8090 \
  -v "$PWD/wow-data:/app/wow-data:ro" \
  azeroth-survivors:local
```

## 设计要点

- **游戏数据不进镜像**:MPQ(约 3.6GB)在宿主机 `wow-data/`,运行时只读挂载,重建镜像零下载。
- **端口只绑宿主机 loopback**:wenilla-host 的 `/data` 接口无鉴权(上游明确警告不可暴露公网),compose 已限定 `127.0.0.1:8090:8090`。
- **网络适配**(本机环境:IPv6 黑洞 + 晚高峰国际 QoS):镜像内 crates.io 走 rsproxy、rustup 走 USTC、GitHub 资产走 gh-proxy、Docker Hub 基础镜像用 daocloud 拉取后 retag;`GH_PROXY` 可用 `--build-arg GH_PROXY=` 覆盖(留空 = 直连)。
- 已完成的 `wow-data/.parts/` 分片按 64MB 索引,换分片大小会作废已有数据。

## 公网/远程给朋友玩(正式方案,已上线)

**正式入口:https://wow.loser.work/?lang=zhcn**(中文界面;账号见下方账号体系;HTTPS 正规证书,WebGPU 可用)

> 界面语言由 `?lang=zhcn`(网页)或 `WOW_LANG=zhcn`(原生)控制,不带参数默认英文。`localization/zhcn` 分支的 i18n 目录覆盖幸存者玩法 UI;魔兽世界内容文字随数据语言(当前 zhCN)。

> 域名说明:Cloudflare 免费通用证书只覆盖 `loser.work` 和 `*.loser.work`(一级子域),二级子域(如 wow.game.loser.work)证书不匹配不可用。当前 `game.loser.work` 与 `wow.loser.work` 两个地址都在线,指向同一游戏。

链路:朋友浏览器 → Cloudflare 边缘(海外,无需备案)→ cloudflared 隧道(Mac,launchd 自启 `com.cloudflare.wow-tunnel`)→ caddy 认证层 8443 → wenilla-host。

- 隧道:`wow-survivors`(ID 7a40dc2a-…,remote-managed),入口规则 `wow.loser.work`、`game.loser.work` → `http://localhost:8443`;
- DNS:`wow` / `game` CNAME → `<tunnel-id>.cfargotunnel.com`(proxied),zone `loser.work` 在 Cloudflare;
- 管理命令:`launchctl kickstart -k gui/$(id -u)/com.cloudflare.wow-tunnel`(重启隧道),日志 `/opt/homebrew/var/log/wow-tunnel.log`;
- CF API Token 存于 `~/.cf_token`(账号级,可随时在 CF 控制台吊销)。

**注意:浏览器必须走 HTTPS 链接**——明文 HTTP(IP:8090/18090)下浏览器禁用 WebGPU,游戏会报 `Failed to create wgpu surface` 崩溃,只能看加载界面。

## 本机直连(自己玩)

浏览器打开 `http://127.0.0.1:8090`(localhost 是受信来源,WebGPU 可用,无需认证)。

## 公网备用通道(明文 HTTP,仅供排查)

认证层 + frp 中继仍在运行:ECS 中继(frpc.toml 的 serverAddr):8090 / :18090 → Mac caddy:8443(账号同上)。因无 HTTPS,WebGPU 不可用,不要用它玩游戏,仅用于排查隧道问题。

**账号体系**(caddy/Caddyfile):wow 为管理员,player1-3 为朋友账号(密码见交付记录,`/tmp/wow_accounts.txt` 留档)。

```bash
# 加账号:生成哈希后追加一行到 caddy/Caddyfile 的 basic_auth 块
docker run --rm caddy:2-alpine caddy hash-password --plaintext '新密码'
# 删账号/改密码:删除或替换对应行,然后重启
docker restart azeroth-survivors-auth
# 查谁在访问(日志含 user_id 字段):
docker logs azeroth-survivors-auth --tail 50
```

ECS 侧防轰炸:`INPUT_direct` 里有单 IP 并发 >16 连接即 REJECT 的规则(`firewall-cmd --direct --get-all-rules` 查看)。

三选一:

1. **Tailscale(推荐,零公网暴露)**:本机 `brew install tailscale` 并登录,朋友装 Tailscale 加入你的网络,访问 `http://<你的tailscale-ip>:8443`。
2. **Cloudflare Tunnel(真公网链接,需一个域名)**:`brew install cloudflared`,建隧道指向 `http://127.0.0.1:8443`;可叠加 Cloudflare Access 邮箱验证。
3. **frp + VPS(国内朋友延迟最好)**:海外 VPS 免备案,`frpc` 的 `local_port` 指向 `8443`。

临时试用可用 `cloudflared tunnel --url http://127.0.0.1:8443`(免账号,随机 trycloudflare.com 域名,国内连通性一般)。

注意:朋友们浏览器同样需要 WebGPU;你的 Mac 需保持开机;家用上行带宽够 3-5 人同时玩。

## 排障

- `Error: opening /app/wow-data/Data` → 数据没下完/没挂载,先跑 `bash fetch-data-v4.sh`。
- 下载卡住不动 → 正常,脚本每 15 分钟强制中止僵死连接自动重试;晚高峰(约 19-23 点)国际带宽可能长时间为 0,过了高峰会自动恢复速度。
- 容器内构建慢 → 检查是否有人改回了直连下载。
