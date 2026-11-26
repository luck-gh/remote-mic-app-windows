# Windows 发布资产矩阵

| 文件 | 用途 | 必需检查 | 通过标准 |
| --- | --- | --- | --- |
| NSIS 安装器 | Release 附件 | 文件名、版本/Build、PE 签名状态、SHA-256 | Preview 记录 Authenticode 状态，Stable 满足代码签名策略；正文的 SHA-256 与附件一致 |
| `latest.json` | Release 附件 | HTTPS URL、版本、平台键、签名内容 | 可解析，下载地址指向同一 Release 的安装器，签名字段为构建 `.sig` 的内容 |
| `.sig` | 构建验签输入 | minisign 签名与对应安装器 | 个人 updater 公钥验签通过 |
| `SHA256SUMS.txt` | 普通 CI 预览构建回执 | CI 安装器摘要 | 摘要与 CI 安装器一致 |
| `build-metadata.json` | 普通 CI 预览构建回执 | source SHA、channel、distribution status | 记录完整构建来源和分发状态 |

上传前打印安装器和清单的大小及 SHA-256。GitHub 附件摘要可用于核对上传的文件。远端已有资产或无法核对响应时停止，保留已有资产。
