# 终态诊断源码审核（未安装）

2026-09-16 原只读 reviewer 独立核对 diagnostic-manifest.json 两份hash：audio5edfb3…/ble6a958…，无P0/P1。owner定向3tests passed，cargo fmt check exit0。只增加host callback计数/时间和正常finish汇总，续期命令/间隔、音频格式/缓存、快捷键及语音生命周期不变。未构建安装包、未安装、未作BUG已修复结论。

字段精度：generation来自host pipeline；audio协议消息自身无session-id。epoch/time能排除明显旧epoch或START前进入callback的元数据，但若旧音频在新START后才首次抵达，仍可能归入新代。它是host callback attribution，不是物理采样归属；reordered_callbacks非0时max_callback_gap是处理时可见上界，不等同完整排序后的真实间隔。decoded与WASAPI accepted分开，送入worker不代表被sink接受；不宣称空中无线丢包。
