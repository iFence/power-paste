# 窗口尺寸自动调整功能

## 功能说明

应用现在支持智能窗口尺寸管理：
- **设置面板**：自动调整为固定的宽矮布局（600x500）
- **主面板**：记住用户手动调整的窗口尺寸，保持用户偏好

## 行为说明

### 切换到设置面板
当用户点击设置按钮时：
1. 保存当前主面板的窗口尺寸
2. 自动调整为设置面板的固定尺寸（600x500）

### 返回主面板
当用户从设置返回主面板时：
1. 恢复之前保存的窗口尺寸
2. 保持用户手动调整的宽度和高度

### 其他路由切换
在主面板和互传面板之间切换时：
- 不做任何窗口尺寸调整
- 完全保持用户当前的窗口大小

## 窗口尺寸配置

### 设置面板 / 互传面板（固定尺寸）
- 设置页：650 × 500
- 互传页：800 × 520（宽度不低于 800，避免默认尺寸就落到页面内部的紧凑布局）
- 进入这两个路由时统一套用上表尺寸，回到主面板时恢复用户保存的主面板尺寸

### 主面板（用户自定义）
- 尊重用户手动调整的尺寸
- 默认初始尺寸：380 × 760（在 tauri.conf.json 中配置）

## 技术实现

### 1. 窗口尺寸管理 Composable

创建了 `src/composables/useWindowSize.js`，负责：
- 监听路由变化
- 保存和恢复主面板的窗口尺寸
- 只在必要时调整窗口尺寸

### 2. 尺寸记忆机制

```javascript
// 保存用户在主面板的窗口尺寸
let savedHomeSize = null

// 从主面板切换到设置面板时保存
savedHomeSize = {
  width: currentSize.width,
  height: currentSize.height,
}

// 从设置面板返回主面板时恢复
await appWindow.setSize({
  type: 'Logical',
  width: savedHomeSize.width,
  height: savedHomeSize.height,
})
```

### 3. CSS 过渡

在 `src/styles/base.css` 中添加了页面切换的淡入淡出效果：
```css
.page-transition-enter-active,
.page-transition-leave-active {
    transition: opacity 180ms ease;
}

.page-transition-enter-from,
.page-transition-leave-to {
    opacity: 0;
}
```

## 使用方式

功能已自动集成到应用中，无需手动配置。用户体验：

1. **首次启动**：窗口使用默认尺寸（380x760）
2. **手动调整**：用户可以随意拖拽调整主面板的窗口大小
3. **打开设置**：窗口自动变为宽矮布局（600x500）
4. **返回主面板**：窗口恢复到用户之前调整的尺寸
5. **切换互传**：窗口保持当前尺寸不变

## 自定义配置

如需调整设置面板的固定尺寸，可修改 `src/composables/useWindowSize.js` 中的 `WINDOW_SIZES` 配置：

```javascript
const WINDOW_SIZES = {
  settings: {
    width: 600,
    height: 500,
  },
}
```

如需调整主面板的默认初始尺寸，可修改 `src-tauri/tauri.conf.json`：

```json
{
  "app": {
    "windows": [
      {
        "width": 380,
        "height": 760,
        "minWidth": 380,
        "minHeight": 600
      }
    ]
  }
}
```

## 注意事项

1. 窗口尺寸调整使用 Tauri 的 `setSize` API
2. 主面板的尺寸保存在内存中，应用重启后会恢复默认尺寸
3. 只有设置面板使用固定尺寸，其他页面都尊重用户的窗口大小
4. 错误会被捕获并记录到控制台，不会影响应用正常使用
