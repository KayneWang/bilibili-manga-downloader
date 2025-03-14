# 🚨 本仓库已停止维护（Archived）
由于哔哩哔哩漫画对脚本检测比较严格，容易造成封号风险，本项目已不再维护，建议使用其他替代方案。如果有兴趣，你可以 Fork 本仓库自行维护。

# 哔哩哔哩漫画下载工具

通过 cli 的形式，将在哔漫已经购买过的漫画下载到本地，未购买的漫画无法下载全部图片.

## 预览

<img src="https://github.com/KayneWang/bilibili-manga-downloader/blob/main/doc/step1.png" alt="step1">
<img src="https://github.com/KayneWang/bilibili-manga-downloader/blob/main/doc/step2.png" alt="step2">

## 使用方法

### 安装

```shell
$ cargo install bili-manga-downloader
$ bili-manga-downloader
```

### 本地构建

1. clone 项目到本地

2. 执行 cargo build --release

3. 运行 bili-manga-downloader

## 拓展

正常情况下，直接运行 bili-manga-downloader，然后根据提示完成设置就ok，如果有特殊需求，可以参考以下方法：

1. 指定漫画搜索

```shell
$ bili-manga-downloader -m 鬼灭之刃
```
2. 指定下载路径

```shell
$ bili-manga-downloader -d xxxxxx
```
