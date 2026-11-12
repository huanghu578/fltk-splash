# splash-page

基于 [FLTK](https://github.com/fltk-rs/fltk-rs) 的轻量级启动画面（splash screen）库。

- 背景图以 **Base64** 字符串内嵌，无需外部资源文件；
- 文本行支持左 / 中 / 右对齐，可自定义颜色、字号、字体族与粗体；
- 文本行可绑定链接：鼠标悬停变手型，点击用系统浏览器打开；
- 由 TOML 配置文件驱动，便于热更新与打包分发；
- 自动探测 Windows / macOS / Linux 上的中文字体，避免中文乱码。

## 安装

```toml
[dependencies]
splash-page = "0.1"
```
运行时需要 FLTK 环境；fltk-rs 默认静态编译 FLTK，无需手动安装 SDK。

## 快速开始
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    splash_page::init("splash.toml")?;
    Ok(())
}
```
### splash.toml 示例：

```toml
enabled = true
duration_ms = 3000
width = 900
height = 600
background_image_base64 = "data:image/png;base64,iVBORw0KGgo..."

[[lines]]
text = "My Application"
color = "#FFFFFF"
font_family = "Consolas"
font_size = 48
bold = true
alignment = "center"

[[lines]]
text = "https://example.com"
color = "#4FC3F7"
font_size = 20
alignment = "center"
link = "https://example.com"
```
### 配置说明
#### 顶层字段
| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `enabled` | bool | 是 | 为 `false` 时 `init` 直接返回，不创建窗口 |
| `duration_ms` | u64 | 是 | 显示时长（毫秒） |
| `width` | u32 | 是 | 窗口宽度（像素） |
| `height` | u32 | 是 | 窗口高度（像素） |
| `background_image_base64` | string | 是 | 背景图 Base64，可带 `data:` 前缀 |
| `lines` | array | 否 | 文本行列表 |

#### [[lines]]
| 字段 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `text` | string | `""` | 文本内容；空串占一行高度但不绘制 |
| `color` | string | `#DDDDDD` | `#RRGGBB` 或 `RRGGBB` |
| `font_family` | string | `""` | 字体族名，经 `Font::by_name` 解析；为空时按 `bold` 自动选中文字体 |
| `font_size` | u32 | `38` | 字号（像素） |
| `alignment` | `"left"` / `"center"` / `"right"` | `"left"` | 水平对齐方式 |
| `bold` | bool | `false` | 是否粗体 |
| `link` | string | 无 | 点击时打开的 URL；缺省则不响应点击 |

font_family 必须是目标系统已安装的字体；未知字体名会回退到 FLTK 默认字体。

#### 支持的图片格式
通过 magic number 自动识别，不依赖扩展名或 MIME 声明：

-PNG
-JPEG
-GIF（仅首帧）
-BMP

背景图在绘制时由 FLTK 拉伸到窗口尺寸，不做预缩放，以节省一份
width × height × 3 的内存。若在低端设备上出现重绘卡顿，可自行预缩放。

#### 行为说明
窗口居中、无边框、模态，背景默认黑色。
duration_ms 到期后仅隐藏窗口，不调用 app::quit()，避免嵌入宿主应用时连带杀掉宿主事件循环。
中文字体通过 Font::load_font 加载后按名称使用，不修改全局 Font::Screen / Font::ScreenBold，不会污染宿主的字体设置。
非法颜色（非 #RRGGBB）会打印告警并回退为白色，而不是静默吞掉配置错误。
单行字体解析优先级：font_family 非空时直接经 Font::by_name 使用；否则按 bold 使用自动探测的中文字体（常规或粗体）；若未找到中文字体，最终回退到 Helvetica / HelveticaBold。

#### 平台字体探测
| 平台 | 常规 | 粗体 |
|---|---|---|
| Windows | simkai / msyh / msyh | simhei / msyhbd |
| macOS | STHeiti Light / PingFang | PingFang |
| Linux | wqy-zenhei / NotoSansCJK-Regular | NotoSansCJK-Bold / wqy-zenhei |

均未找到时回退到 Helvetica / HelveticaBold。

### 示例
sh
cargo run --example basic -- path/to/splash.toml
## 许可证
MIT OR Apache-2.0