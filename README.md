# 彝文输入法 (Yi IME)

一个基于 Rust 和原生 Windows API 开发的现代化标准彝文输入法，提供智能拼音输入、联想和候选词显示、全局快捷键支持。

## 演示


https://github.com/user-attachments/assets/47afdae6-fba6-461f-b66a-713a29f0e61f

> 👇 《世界人权宣言》第一条
> 使用 HTML 注音模式输入


https://github.com/user-attachments/assets/ef61cd01-040b-4185-80f7-ffd312dde940



## 特性

- 直接键入彝语拼音即可得到彝文字母（包括彝文部首）
- 对有歧义的拼音序列进行枚举，并用数字选中
- 可从声母联想音节、从单音节联想声调
- 适应系统深浅色主题，运行时位于系统托盘中，用 F4 键切换彝文输入与常规输入模式
- 提供彝文与拼音混排的快捷输入（包括 HTML <ruby>注<rt>zhu</rt></ruby><ruby>音<rt>yin</rt></ruby>）
- 提供多语言交互信息

## 构建

<details>
<summary>展开</summary>

### 系统要求

- Windows 10/11 (x64)
- Visual Studio Build Tools 或 Visual Studio (用于 C++ 编译)
- Rust Toolchain (1.70+)

### 从源码构建

1. **克隆仓库**

   ```bash
   git clone https://github.com/your-username/yi-ime.git
   cd yi-ime
   ```
2. **安装依赖**

   ```bash
   # 确保已安装 Rust
   rustup update

   # 安装 Windows 构建工具（如果尚未安装）
   # 下载并安装 Visual Studio Build Tools
   ```
3. **构建项目**

   ```bash
   cargo build --release
   ```
4. **运行输入法**

   ```bash
   cargo run --release
   ```

</details>


## 预编译版本

从 [Releases 页面](https://github.com/tanpero/yi/releases) 下载最新的预编译版本。

## 使用

### 基本方法

1. 双击运行 `yi-global.exe`，按 F4 进入彝文输入模式
2. 在任意文本框或编辑器中开始输入拼音（替字符 ꀕ 使用 `w` 表示）
3. 使用数字键 1-9 选择候选词或按空格键选中首个候选词
4. 使用退格键清除输入框中的拼音字母，或使用 `Esc` 键退出输入


## License

[MIT LICENSE](LICENSE)

## 作者

[Camille Dolma](https://github.com/tanpero)







