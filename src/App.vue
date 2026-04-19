<template>
  <div class="app-container" :class="{ 'dark-mode': isDarkMode }">
    <!-- 顶部导航栏 -->
    <el-card class="navbar" shadow="never">
      <div class="navbar-content">
        <div class="app-title">
          <el-icon :size="28" class="title-icon"><Promotion /></el-icon>
          <div class="title-text">
            <h1>NUCtool 智能风扇控制</h1>
            <p>Professional Fan Management System</p>
          </div>
        </div>
        <div class="status-bar">
          <el-tag :type="fanStatus.type" effect="dark" :class="{ 'pulse-tag': isRunning }">
            <el-icon v-if="isRunning" class="spin-icon"><Loading /></el-icon>
            {{ fanStatus.text }}
          </el-tag>
          <el-tag :type="configStatus.type" effect="dark">
            {{ configStatus.text }}
          </el-tag>
          <div class="autostart-toggle">
            <span class="autostart-label">开机自启</span>
            <el-switch
              v-model="autostartEnabled"
              @change="toggleAutostart"
              inline-prompt
              active-text="开"
              inactive-text="关"
            />
          </div>
          <el-tooltip content="切换深色模式" placement="bottom">
            <el-button :icon="isDarkMode ? Sunny : Moon" circle size="small" @click="isDarkMode = !isDarkMode" />
          </el-tooltip>
        </div>
      </div>
    </el-card>

    <!-- 过温警告横幅 -->
    <el-alert
      v-if="isOverheat"
      title="⚠️ 过温警告：CPU或GPU温度超过85°C，风扇已加速散热"
      type="error"
      effect="dark"
      :closable="false"
      class="overheat-alert"
      show-icon
    />

    <!-- 监控指标 -->
    <el-row :gutter="12" class="metrics-grid">
      <el-col :span="6">
        <el-card shadow="hover" class="metric-card">
          <div class="metric-header">
            <el-icon :size="18"><Sunny /></el-icon>
            <span class="metric-label">CPU 温度</span>
            <div class="metric-badge" :class="getTempLevel(metrics.cpuTemp)">
              {{ getTempLevelText(metrics.cpuTemp) }}
            </div>
          </div>
          <div class="metric-value-container">
            <span class="metric-value" :style="{ color: getTempColor(metrics.cpuTemp) }">
              {{ metrics.cpuTemp || '--' }}
            </span>
            <span class="metric-unit">°C</span>
          </div>
          <el-progress
            :percentage="metrics.cpuTemp ? Math.min(metrics.cpuTemp, 100) : 0"
            :color="getTempColor(metrics.cpuTemp)"
            :stroke-width="4"
            :show-text="false"
            class="metric-progress"
          />
          <div class="metric-trend">
            <span v-for="(v, i) in cpuTempHistory" :key="i" class="spark-bar"
              :style="{ height: (v/100*20)+'px', background: getTempColor(v) }"></span>
          </div>
        </el-card>
      </el-col>

      <el-col :span="6">
        <el-card shadow="hover" class="metric-card">
          <div class="metric-header">
            <el-icon :size="18"><Cpu /></el-icon>
            <span class="metric-label">GPU 温度</span>
            <div class="metric-badge" :class="getTempLevel(metrics.gpuTemp)">
              {{ getTempLevelText(metrics.gpuTemp) }}
            </div>
          </div>
          <div class="metric-value-container">
            <span class="metric-value" :style="{ color: getTempColor(metrics.gpuTemp) }">
              {{ metrics.gpuTemp || '--' }}
            </span>
            <span class="metric-unit">°C</span>
          </div>
          <el-progress
            :percentage="metrics.gpuTemp ? Math.min(metrics.gpuTemp, 100) : 0"
            :color="getTempColor(metrics.gpuTemp)"
            :stroke-width="4"
            :show-text="false"
            class="metric-progress"
          />
          <div class="metric-trend">
            <span v-for="(v, i) in gpuTempHistory" :key="i" class="spark-bar"
              :style="{ height: (v/100*20)+'px', background: getTempColor(v) }"></span>
          </div>
        </el-card>
      </el-col>

      <el-col :span="6">
        <el-card shadow="hover" class="metric-card">
          <div class="metric-header">
            <el-icon :size="18"><WindPower /></el-icon>
            <span class="metric-label">CPU 风扇</span>
            <div class="metric-badge badge-blue">{{ metrics.cpuFan ? '运转中' : '待机' }}</div>
          </div>
          <div class="metric-value-container">
            <span class="metric-value fan-value">{{ metrics.cpuFan || '--' }}</span>
            <span class="metric-unit">RPM</span>
          </div>
          <el-progress
            :percentage="metrics.cpuFan ? Math.min(Math.round(metrics.cpuFan / 60), 100) : 0"
            color="#409EFF"
            :stroke-width="4"
            :show-text="false"
            class="metric-progress"
          />
          <div class="metric-trend">
            <span v-for="(v, i) in cpuFanHistory" :key="i" class="spark-bar"
              :style="{ height: (v/6000*20)+'px', background: '#409EFF' }"></span>
          </div>
        </el-card>
      </el-col>

      <el-col :span="6">
        <el-card shadow="hover" class="metric-card">
          <div class="metric-header">
            <el-icon :size="18"><CircleCheck /></el-icon>
            <span class="metric-label">GPU 风扇</span>
            <div class="metric-badge badge-green">{{ metrics.gpuFan ? '运转中' : '待机' }}</div>
          </div>
          <div class="metric-value-container">
            <span class="metric-value fan-value">{{ metrics.gpuFan || '--' }}</span>
            <span class="metric-unit">RPM</span>
          </div>
          <el-progress
            :percentage="metrics.gpuFan ? Math.min(Math.round(metrics.gpuFan / 60), 100) : 0"
            color="#67C23A"
            :stroke-width="4"
            :show-text="false"
            class="metric-progress"
          />
          <div class="metric-trend">
            <span v-for="(v, i) in gpuFanHistory" :key="i" class="spark-bar"
              :style="{ height: (v/6000*20)+'px', background: '#67C23A' }"></span>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 快捷调节：模式 + 强度 + 应用 -->
    <el-card class="quick-card" shadow="hover">
      <div class="quick-row">
        <div class="quick-block">
          <div class="quick-label">
            <el-icon><Setting /></el-icon>
            <span>运行模式</span>
          </div>
          <el-radio-group v-model="mode" size="default">
            <el-radio-button
              v-for="opt in modeOptions"
              :key="opt.value"
              :label="opt.value"
            >
              <el-icon style="margin-right:4px">
                <component :is="opt.icon" />
              </el-icon>
              {{ opt.label }}
            </el-radio-button>
          </el-radio-group>
        </div>
        <div class="quick-block stretch">
          <div class="quick-label">
            <span>
              <el-icon><Filter /></el-icon>
              强度调节
            </span>
            <el-tag size="small" effect="plain" type="primary">{{ intensity }}%</el-tag>
          </div>
          <el-slider v-model="intensity" :min="0" :max="100" :step="1"
            :marks="{ 0: '静', 50: '均衡', 100: '性能' }" />
          <div class="suggest-box" v-if="suggestedIntensity !== null">
            建议: <strong>{{ suggestedIntensity }}%</strong>
            <el-button size="small" type="warning" @click="applySuggested" style="margin-left:8px">应用建议</el-button>
          </div>
        </div>
        <div class="quick-block stretch">
          <div class="quick-label">
            <span>
              <el-icon><WindPower /></el-icon>
              恒速设置
            </span>
            <el-tag size="small" effect="plain" type="success">{{ constantFanSpeed }}%</el-tag>
          </div>
          <el-switch
            v-model="constantSpeedEnabled"
            inline-prompt
            active-text="开"
            inactive-text="关"
          />
          <el-slider
            v-model="constantFanSpeed"
            :min="0"
            :max="100"
            :step="1"
            :disabled="!constantSpeedEnabled"
            :marks="{ 20: '静音', 50: '常用', 80: '高转' }"
          />
          <div class="suggest-box" v-if="constantSpeedEnabled">
            启用后风扇将固定在 <strong>{{ constantFanSpeed }}%</strong> 转速执行
          </div>
        </div>
        <div class="quick-actions">
          <el-button type="primary" @click="applyQuickMode" :loading="isLoading" :icon="Lightning">
            快速应用
          </el-button>
          <el-button type="success" @click="applyCustomCurve" :loading="isLoading" :icon="Edit">
            自定义曲线
          </el-button>
          <el-divider direction="vertical" />
          <el-tooltip :content="advancedVisible ? '收起高级视图' : '展开高级视图'" placement="top">
            <el-switch
              v-model="advancedVisible"
              inline-prompt
              active-text="高级"
              inactive-text="简洁"
            />
          </el-tooltip>
        </div>
      </div>
    </el-card>

    <!-- 图表区域（高级模式可见） -->
    <el-row v-show="advancedVisible" :gutter="12" class="charts-container">
      <el-col :span="12">
        <el-card shadow="hover" class="chart-card">
          <template #header>
            <div class="chart-header">
              <el-icon :size="18" color="#409EFF"><TrendCharts /></el-icon>
              <div>
                <span class="chart-title">CPU 风扇曲线</span>
                <p class="chart-subtitle">拖动节点调整温度-转速映射</p>
              </div>
              <el-tag size="small" type="primary" style="margin-left:auto">
                当前 {{ metrics.cpuTemp || '--' }}°C
              </el-tag>
            </div>
          </template>
          <div class="chart-wrapper">
            <canvas ref="leftFanChart"></canvas>
          </div>
        </el-card>

        <el-card shadow="hover" class="chart-card" style="margin-top: 12px;">
          <template #header>
            <div class="chart-header">
              <el-icon :size="18" color="#67C23A"><TrendCharts /></el-icon>
              <div>
                <span class="chart-title">GPU 风扇曲线</span>
                <p class="chart-subtitle">拖动节点调整温度-转速映射</p>
              </div>
              <el-tag size="small" type="success" style="margin-left:auto">
                当前 {{ metrics.gpuTemp || '--' }}°C
              </el-tag>
            </div>
          </template>
          <div class="chart-wrapper">
            <canvas ref="rightFanChart"></canvas>
          </div>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card shadow="hover" class="chart-card">
          <template #header>
            <div class="chart-header">
              <el-icon :size="18" color="#E6A23C"><DataLine /></el-icon>
              <div>
                <span class="chart-title">风扇转速监控</span>
                <p class="chart-subtitle">实时转速变化（最近60秒）</p>
              </div>
              <div style="margin-left:auto;display:flex;gap:6px">
                <el-tag size="small" type="primary">CPU {{ metrics.cpuFan || 0 }} RPM</el-tag>
                <el-tag size="small" type="success">GPU {{ metrics.gpuFan || 0 }} RPM</el-tag>
              </div>
            </div>
          </template>
          <div class="chart-wrapper">
            <canvas ref="speedChart"></canvas>
          </div>
        </el-card>

        <el-card shadow="hover" class="chart-card" style="margin-top: 12px;">
          <template #header>
            <div class="chart-header">
              <el-icon :size="18" color="#F56C6C"><DataLine /></el-icon>
              <div>
                <span class="chart-title">温度历史</span>
                <p class="chart-subtitle">实时温度变化（最近60秒）</p>
              </div>
              <div style="margin-left:auto;display:flex;gap:6px">
                <el-tag size="small" type="danger">CPU {{ metrics.cpuTemp || 0 }}°C</el-tag>
                <el-tag size="small" type="warning">GPU {{ metrics.gpuTemp || 0 }}°C</el-tag>
              </div>
            </div>
          </template>
          <div class="chart-wrapper">
            <canvas ref="tempChart"></canvas>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 控制按钮 -->
    <div class="controls">
      <el-button
        :type="isRunning ? 'danger' : 'primary'"
        size="large"
        @click="toggleFanControl"
        :loading="isLoading"
        class="main-control-btn"
      >
        <el-icon><VideoPlay v-if="!isRunning" /><VideoPause v-else /></el-icon>
        {{ isRunning ? '停止控制' : '启动控制' }}
      </el-button>
      <el-divider direction="vertical" style="height:36px" />
      <el-button size="large" @click="loadConfig" :loading="isLoading" :icon="Download">
        加载配置
      </el-button>
      <el-button size="large" @click="saveConfig" :loading="isLoading" :icon="Upload">
        保存配置
      </el-button>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Promotion, Sunny, Cpu, WindPower, CircleCheck,
  TrendCharts, DataLine, VideoPlay, VideoPause, Download, Upload,
  Loading, Moon, Setting, Filter, Edit, Lightning
} from '@element-plus/icons-vue'
import Chart from 'chart.js/auto'
import dragDataPlugin from 'chartjs-plugin-dragdata'

Chart.register(dragDataPlugin)

const { listen } = window.__TAURI__.event

// 状态管理
const isRunning = ref(false)
const isLoading = ref(false)
const autostartEnabled = ref(false)
const isDarkMode = ref(false)

const fanStatus = reactive({ type: 'info', text: '风扇待机' })
const configStatus = reactive({ type: 'info', text: '配置未加载' })

const metrics = reactive({ cpuTemp: null, gpuTemp: null, cpuFan: null, gpuFan: null })

// 迷你历史数据（sparkline，保留20个点）
const SPARK_LEN = 20
const cpuTempHistory = ref(Array(SPARK_LEN).fill(0))
const gpuTempHistory = ref(Array(SPARK_LEN).fill(0))
const cpuFanHistory  = ref(Array(SPARK_LEN).fill(0))
const gpuFanHistory  = ref(Array(SPARK_LEN).fill(0))

// 过温计算属性
const isOverheat = computed(() => metrics.cpuTemp > 85 || metrics.gpuTemp > 85)

const isTauriReady = () => !!(window.__TAURI__ && window.__TAURI__.core)

// 快捷模式
const mode = ref('auto')
const intensity = ref(50)
const constantSpeedEnabled = ref(false)
const constantFanSpeed = ref(60)
const usageHistory = ref([])

// Load/save usage history from localStorage
const USAGE_KEY = 'nuctool_quick_usage'

function loadUsageHistory() {
  try {
    const raw = localStorage.getItem(USAGE_KEY)
    if (raw) usageHistory.value = JSON.parse(raw)
  } catch (e) { console.warn('无法加载使用历史', e) }
}

function saveUsageHistory() {
  try {
    localStorage.setItem(USAGE_KEY, JSON.stringify(usageHistory.value.slice(-200))) // keep last 200
  } catch (e) { console.warn('无法保存使用历史', e) }
}

// suggested intensity based on recent history for current mode
const suggestedIntensity = computed(() => {
  const modeHist = usageHistory.value.filter(h => h.mode === mode.value)
  if (!modeHist.length) return null
  const last = modeHist.slice(-20) // use last 20
  return Math.round(last.reduce((s, r) => s + r.intensity, 0) / last.length)
})

// 根据模式+强度生成曲线
const buildCurveFromMode = (labels, selectedMode, k) => {
  const clamp = (v, a, b) => Math.max(a, Math.min(b, v))
  return labels.map((t) => {
    const temp = Number(t)
    const base = ((temp - 30) / 60) * 70 + 20
    const shaped = clamp(base, 10, 95)
    let offset = 0, slope = 1
    if (selectedMode === 'silent')      { offset = -14; slope = 0.82 }
    else if (selectedMode === 'balanced') { offset = -4; slope = 1.0 }
    else if (selectedMode === 'performance') { offset = 8; slope = 1.12 }
    else if (selectedMode === 'auto') { offset = -6 + 12 * k; slope = 0.95 + 0.22 * k }
    const highTempGain = clamp((temp - 60) / 30, 0, 1)
    const speed = clamp(shaped * slope + offset + k * 18 + k * 20 * highTempGain, 10, 100)
    return { temperature: temp, speed: Math.round(speed) }
  })
}

// 快捷应用模式
const applyQuickMode = async () => {
  if (!leftFanChartInstance || !rightFanChartInstance) { ElMessage.warning('图表未初始化'); return }
  isLoading.value = true
  try {
    const k = intensity.value / 100
    const fanData = {
      left_fan: buildCurveFromMode(leftFanChartInstance.data.labels, mode.value, k),
      right_fan: buildCurveFromMode(rightFanChartInstance.data.labels, mode.value, k),
      control: {
        constant_speed_enabled: constantSpeedEnabled.value,
        constant_speed: constantFanSpeed.value,
      }
    }
    leftFanChartInstance.data.datasets[0].data  = fanData.left_fan.map(p => p.speed)
    rightFanChartInstance.data.datasets[0].data = fanData.right_fan.map(p => p.speed)
    leftFanChartInstance.update()
    rightFanChartInstance.update()
    if (isRunning.value) {
      await window.__TAURI__.core.invoke('stop_fan_control')
      await new Promise(resolve => setTimeout(resolve, 500))
    }
    await window.__TAURI__.core.invoke('start_fan_control', { fanData })
    isRunning.value = true
    fanStatus.type = 'success'; fanStatus.text = '风扇运行中'
    ElMessage.success('已应用快捷模式')

    // 记录使用历史（异步，不阻塞 UI）
    try {
      usageHistory.value.push({ ts: Date.now(), mode: mode.value, intensity: intensity.value })
      saveUsageHistory()
    } catch (e) { console.warn('记录使用历史失败', e) }
  } catch (error) {
    ElMessage.error('应用失败: ' + error)
  } finally {
    isLoading.value = false
  }
}

// 应用当前自定义曲线（使用图表手动调整的数据）
const applyCustomCurve = async () => {
  if (!leftFanChartInstance || !rightFanChartInstance) { ElMessage.warning('图表未初始化'); return }
  isLoading.value = true
  try {
    const fanData = {
      left_fan: leftFanChartInstance.data.labels.map((temp, i) => ({
        temperature: temp, speed: leftFanChartInstance.data.datasets[0].data[i]
      })),
      right_fan: rightFanChartInstance.data.labels.map((temp, i) => ({
        temperature: temp, speed: rightFanChartInstance.data.datasets[0].data[i]
      }))
    }
    if (isRunning.value) {
      await window.__TAURI__.core.invoke('stop_fan_control')
      await new Promise(resolve => setTimeout(resolve, 500))
    }
    await window.__TAURI__.core.invoke('start_fan_control', { fanData })
    isRunning.value = true
    fanStatus.type = 'success'; fanStatus.text = '风扇运行中'
    ElMessage.success('自定义曲线已应用')
  } catch (error) {
    ElMessage.error('应用失败: ' + error)
  } finally {
    isLoading.value = false
  }
}

// canvas refs and chart instances
const leftFanChart = ref(null)
const rightFanChart = ref(null)
const speedChart = ref(null)
const tempChart = ref(null)
let leftFanChartInstance = null
let rightFanChartInstance = null
let speedChartInstance = null
let tempChartInstance = null

// 初始化图表
const initCharts = () => {
  const baseScales = (xTitle, yTitle, yMin, yMax) => ({
    x: {
      grid: { color: 'rgba(0,0,0,0.06)' },
      ticks: { color: '#606266', font: { size: 11 } },
      title: { display: true, text: xTitle, color: '#909399', font: { size: 11 } }
    },
    y: {
      grid: { color: 'rgba(0,0,0,0.06)' },
      ticks: { color: '#606266', font: { size: 11 } },
      title: { display: true, text: yTitle, color: '#909399', font: { size: 11 } },
      min: yMin, max: yMax
    }
  })

  const baseOptions = (xTitle, yTitle, yMin, yMax, extra = {}) => ({
    responsive: true,
    maintainAspectRatio: false,
    animation: false,
    plugins: {
      legend: { display: true, labels: { color: '#606266', font: { size: 11 }, boxWidth: 12 } },
      tooltip: { backgroundColor: 'rgba(48,49,51,0.9)', titleColor: '#fff', bodyColor: '#ddd' },
      ...extra
    },
    scales: baseScales(xTitle, yTitle, yMin, yMax)
  })

  const curveLabels = Array.from({ length: 15 }, (_, i) => (i + 30) + Math.round(i * 4))

  leftFanChartInstance = new Chart(leftFanChart.value, {
    type: 'line',
    data: {
      labels: curveLabels,
      datasets: [{
        label: 'CPU风扇速度 (%)',
        data: Array(15).fill(50),
        borderColor: '#409EFF',
        backgroundColor: 'rgba(64,158,255,0.08)',
        borderWidth: 2.5,
        pointRadius: 5,
        pointHoverRadius: 8,
        pointBackgroundColor: '#409EFF',
        tension: 0.3,
        fill: true
      }]
    },
    options: baseOptions('温度 (°C)', '风扇速度 (%)', 0, 100, {
      dragData: { round: 0, showTooltip: true, onDrag: (e, di, idx, val) => val >= 0 && val <= 100 }
    })
  })

  rightFanChartInstance = new Chart(rightFanChart.value, {
    type: 'line',
    data: {
      labels: curveLabels,
      datasets: [{
        label: 'GPU风扇速度 (%)',
        data: Array(15).fill(50),
        borderColor: '#67C23A',
        backgroundColor: 'rgba(103,194,58,0.08)',
        borderWidth: 2.5,
        pointRadius: 5,
        pointHoverRadius: 8,
        pointBackgroundColor: '#67C23A',
        tension: 0.3,
        fill: true
      }]
    },
    options: baseOptions('温度 (°C)', '风扇速度 (%)', 0, 100, {
      dragData: { round: 0, showTooltip: true, onDrag: (e, di, idx, val) => val >= 0 && val <= 100 }
    })
  })

  const historyLabels = Array.from({ length: 21 }, (_, i) => i * 3 + 's')

  speedChartInstance = new Chart(speedChart.value, {
    type: 'line',
    data: {
      labels: historyLabels,
      datasets: [
        { label: 'CPU风扇', data: Array(21).fill(0), borderColor: '#409EFF', backgroundColor: 'rgba(64,158,255,0.08)', borderWidth: 2, pointRadius: 0, tension: 0.4, fill: true },
        { label: 'GPU风扇', data: Array(21).fill(0), borderColor: '#67C23A', backgroundColor: 'rgba(103,194,58,0.08)', borderWidth: 2, pointRadius: 0, tension: 0.4, fill: true }
      ]
    },
    options: baseOptions('时间', '转速 (RPM)', 0, 6000)
  })

  tempChartInstance = new Chart(tempChart.value, {
    type: 'line',
    data: {
      labels: historyLabels,
      datasets: [
        { label: 'CPU温度', data: Array(21).fill(0), borderColor: '#F56C6C', backgroundColor: 'rgba(245,108,108,0.08)', borderWidth: 2, pointRadius: 0, tension: 0.4, fill: true },
        { label: 'GPU温度', data: Array(21).fill(0), borderColor: '#E6A23C', backgroundColor: 'rgba(230,162,60,0.08)',  borderWidth: 2, pointRadius: 0, tension: 0.4, fill: true }
      ]
    },
    options: baseOptions('时间', '温度 (°C)', 0, 100)
  })
}

const pushSpark = (arr, val, max = SPARK_LEN) => {
  arr.push(val ?? 0)
  if (arr.length > max) arr.shift()
}

const updateMetrics = (data) => {
  metrics.cpuTemp = data.left_temp
  metrics.gpuTemp = data.right_temp
  metrics.cpuFan  = data.left_fan_speed
  metrics.gpuFan  = data.right_fan_speed

  pushSpark(cpuTempHistory.value, data.left_temp)
  pushSpark(gpuTempHistory.value, data.right_temp)
  pushSpark(cpuFanHistory.value,  data.left_fan_speed)
  pushSpark(gpuFanHistory.value,  data.right_fan_speed)

  if (speedChartInstance) {
    speedChartInstance.data.datasets[0].data.push(data.left_fan_speed)
    speedChartInstance.data.datasets[0].data.shift()
    speedChartInstance.data.datasets[1].data.push(data.right_fan_speed)
    speedChartInstance.data.datasets[1].data.shift()
    speedChartInstance.update('none')
  }
  if (tempChartInstance) {
    tempChartInstance.data.datasets[0].data.push(data.left_temp)
    tempChartInstance.data.datasets[0].data.shift()
    tempChartInstance.data.datasets[1].data.push(data.right_temp)
    tempChartInstance.data.datasets[1].data.shift()
    tempChartInstance.update('none')
  }
}

const toggleFanControl = async () => {
  isLoading.value = true
  try {
    if (!isRunning.value) {
      const fanData = {
        left_fan: leftFanChartInstance.data.labels.map((temp, i) => ({
          temperature: temp, speed: leftFanChartInstance.data.datasets[0].data[i]
        })),
        right_fan: rightFanChartInstance.data.labels.map((temp, i) => ({
          temperature: temp, speed: rightFanChartInstance.data.datasets[0].data[i]
        })),
        control: {
          constant_speed_enabled: constantSpeedEnabled.value,
          constant_speed: constantFanSpeed.value,
        }
      }
      await window.__TAURI__.core.invoke('start_fan_control', { fanData })
      isRunning.value = true
      fanStatus.type = 'success'; fanStatus.text = '风扇运行中'
      ElMessage.success('风扇控制已启动')
    } else {
      await window.__TAURI__.core.invoke('stop_fan_control')
      isRunning.value = false
      fanStatus.type = 'info'; fanStatus.text = '风扇待机'
      ElMessage.info('风扇控制已停止')
    }
  } catch (error) {
    ElMessage.error('操作失败: ' + error)
  } finally {
    isLoading.value = false
  }
}

const loadConfig = async () => {
  isLoading.value = true
  try {
    const fanData = await window.__TAURI__.core.invoke('load_fan_config')
    leftFanChartInstance.data.datasets[0].data  = fanData.left_fan.map(p => p.speed)
    rightFanChartInstance.data.datasets[0].data = fanData.right_fan.map(p => p.speed)
    leftFanChartInstance.update()
    rightFanChartInstance.update()
    configStatus.type = 'success'; configStatus.text = '配置已加载'
    ElMessage.success('配置加载成功')
  } catch (error) {
    configStatus.type = 'warning'; configStatus.text = '配置加载失败'
    ElMessage.warning('配置加载失败: ' + error)
  } finally {
    isLoading.value = false
  }
}

const saveConfig = async () => {
  isLoading.value = true
  try {
    const fanData = {
      left_fan: leftFanChartInstance.data.labels.map((temp, i) => ({
        temperature: temp, speed: leftFanChartInstance.data.datasets[0].data[i]
      })),
      right_fan: rightFanChartInstance.data.labels.map((temp, i) => ({
        temperature: temp, speed: rightFanChartInstance.data.datasets[0].data[i]
      })),
      control: {
        constant_speed_enabled: constantSpeedEnabled.value,
        constant_speed: constantFanSpeed.value,
      }
    }
    await window.__TAURI__.core.invoke('save_fan_config', { fanData })
    configStatus.type = 'success'; configStatus.text = '配置已保存'
    ElMessage.success('配置保存成功')
    setTimeout(() => { configStatus.text = '配置已加载' }, 3000)
  } catch (error) {
    ElMessage.error('配置保存失败: ' + error)
  } finally {
    isLoading.value = false
  }
}

const setupListener = async () => {
  await window.__TAURI__.core.invoke('get_fan_speeds')
  await listen('get-fan-speeds', (event) => { updateMetrics(event.payload) })
}

const toggleAutostart = async () => {
  if (!isTauriReady()) {
    ElMessage.error('Tauri 未就绪，无法设置开机自启')
    autostartEnabled.value = !autostartEnabled.value
    return
  }
  try {
    if (autostartEnabled.value) {
      await window.__TAURI__.core.invoke('plugin:autostart|enable')
      ElMessage.success('开机自启动已启用')
    } else {
      await window.__TAURI__.core.invoke('plugin:autostart|disable')
      ElMessage.info('开机自启动已禁用')
    }
    await refreshAutostartStatus()
  } catch (error) {
    autostartEnabled.value = !autostartEnabled.value
    ElMessage.error('设置开机自启失败: ' + error)
  }
}

// remove unused applySuggested helper to silence linter warnings
const applySuggested = async () => {
  if (suggestedIntensity.value !== null) {
    intensity.value = suggestedIntensity.value
    await applyQuickMode()
  } else {
    ElMessage.info('暂无建议')
  }
}

onMounted(async () => {
  loadUsageHistory()
  await nextTick()
  initCharts()
  await setupListener()
  if (isTauriReady()) {
    try {
      const enabled = await refreshAutostartStatus()
      if (enabled) {
        await loadConfig()
        setTimeout(() => toggleFanControl(), 1000)
      }
    } catch (error) {
      console.error('❌ 检查开机自启状态失败:', error)
    }
  }
})

onUnmounted(() => {
  [leftFanChartInstance, rightFanChartInstance, speedChartInstance, tempChartInstance]
    .forEach(c => c?.destroy())
})
</script>

<style scoped>
/* LoL 2025 inspired theme - cinematic dark gradients, glass panels, neon rim lighting */
@import url('https://fonts.googleapis.com/css2?family=Inter:wght@300;400;600;700;800&display=swap');
:root{
  --bg-1: linear-gradient(180deg, #05060a 0%, #0b1020 60%);
  --panel-bg: rgba(255,255,255,0.03);
  --glass-border: rgba(255,255,255,0.06);
  --accent-1: #8b5cf6; /* purple */
  --accent-2: #00e5ff; /* cyan */
  --accent-3: #ffb86b; /* gold */
  --muted: #b8c5d6;
  --text-main: #f0f5ff;
  --text-secondary: #d0dbe8;
  --soft-shadow: 0 6px 24px rgba(2,6,23,0.6);
}

.app-container {
  padding: 20px;
  font-family: 'Inter', system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial;
  background: var(--bg-1);
  min-height: 100vh;
  color: var(--text-main);
  -webkit-font-smoothing:antialiased;
  transition: background 0.35s ease, color 0.35s ease;
}

/* Glass card base */
.el-card {
  background: linear-gradient(180deg, rgba(255,255,255,0.02), rgba(255,255,255,0.015));
  border: 1px solid var(--glass-border);
  border-radius: 12px;
  box-shadow: var(--soft-shadow);
  backdrop-filter: blur(8px) saturate(110%);
}

/* navbar */
.navbar { margin-bottom: 18px; padding: 10px 12px; }
.navbar-content { display:flex; align-items:center; gap:16px; justify-content:space-between; }
.title-text h1{ font-size:20px; margin:0; color:var(--text-main); letter-spacing:0.6px; font-weight:700 }
.title-text p{ margin:2px 0 0 0; color:var(--muted); font-size:12px }
.title-icon { width:40px; height:40px; display:flex; align-items:center; justify-content:center; border-radius:8px; background:linear-gradient(120deg,var(--accent-1), var(--accent-2)); box-shadow:0 6px 18px rgba(138,92,246,0.14); }

.status-bar { display:flex; gap:10px; align-items:center }
.el-tag { background: linear-gradient(90deg, rgba(255,255,255,0.02), rgba(255,255,255,0.01)); border: 1px solid rgba(255,255,255,0.03); color:var(--text-main); }
.pulse-tag{ animation: glow 1.6s ease-in-out infinite; border-radius:10px }
@keyframes glow{ 0%{ box-shadow:0 0 0 0 rgba(139,92,246,0.15) } 50%{ box-shadow:0 0 28px 6px rgba(0,229,255,0.04) } 100%{ box-shadow:0 0 0 0 rgba(139,92,246,0.06) } }

/* overheat banner - sharper and prominent */
.overheat-alert{ background: linear-gradient(90deg, rgba(245,92,92,0.12), rgba(255,140,92,0.06)); border-left:4px solid #ff6b6b; color:#fff; margin-bottom:14px }

/* metrics grid */
.metrics-grid{ margin-bottom:14px }
.metric-card{ padding:12px }
.metric-header{ display:flex; align-items:center; gap:8px; margin-bottom:8px; color:var(--text-secondary) }
.metric-label{ font-size:11px; font-weight:600; color:var(--text-secondary); text-transform:none }
.metric-value{ font-size:32px; font-weight:800; color:var(--text-main); transition: color 0.45s }
.metric-unit{ color:var(--muted); font-size:13px }
.metric-badge{ font-weight:700; font-size:11px; padding:4px 8px; border-radius:999px }
.badge-green{ background: linear-gradient(90deg,#0f5132 0%, #154d3f 100%); color:#c7ffd8 }
.badge-blue{ background: linear-gradient(90deg,#062f4f 0%, #083b67 100%); color:#cfeaff }

/* progress visual tweaks */
.metric-progress .el-progress-bar__outer{ background: rgba(255,255,255,0.02); border-radius:10px }
.metric-progress .el-progress__inner{ border-radius:10px }

/* spark bars - neon accent */
.metric-trend{ display:flex; gap:4px; height:24px }
.spark-bar{ border-radius:2px; transition:height 0.35s ease, background 0.35s ease }

/* quick card */
.quick-card{ padding:12px; margin-bottom:14px }
.quick-row{ display:flex; gap:14px; align-items:center; flex-wrap:wrap }
.quick-block{ min-width:220px }
.quick-label{ display:flex; align-items:center; justify-content:space-between; gap:8px; color:var(--text-secondary) }
.el-button--primary{ background: linear-gradient(90deg, var(--accent-1), var(--accent-2)); border:none; box-shadow: 0 8px 28px rgba(139,92,246,0.14); }
.el-button--success{ background: linear-gradient(90deg,#2ecc71, #66d08f); border:none }
.el-button{ color:var(--text-main); font-weight:700 }

/* charts */
.chart-card{ height: calc(50vh - 110px); min-height:240px; padding:8px }
.chart-header{ display:flex; align-items:center; gap:10px }
.chart-title{ color:var(--text-main); font-weight:700 }
.chart-subtitle{ color:var(--text-secondary); font-size:12px }
.chart-wrapper{ height:100%; position:relative; }
.chart-wrapper canvas{ width:100% !important; height:100% !important }

/* controls */
.controls{ display:flex; justify-content:center; gap:12px; align-items:center; padding-top:8px }
.main-control-btn{ min-width:160px; border-radius:12px; padding:12px 18px }
.el-divider--vertical{ height:42px }

/* dark-mode tweaks */
.dark-mode{ background: linear-gradient(180deg,#03040a 0%, #071025 80%) }
.dark-mode .el-card{ background: linear-gradient(180deg, rgba(255,255,255,0.02), rgba(255,255,255,0.01)); border:1px solid rgba(255,255,255,0.04) }
.dark-mode .title-text h1, .dark-mode .metric-value{ color:var(--text-main) }
.dark-mode .metric-header, .dark-mode .metric-label, .dark-mode .chart-subtitle, .dark-mode .quick-label{ color:var(--text-secondary) }

/* micro interactions */
.el-card:hover{ transform: translateY(-4px); transition: transform 0.25s ease }
.el-button:hover{ transform: translateY(-2px) }
.spark-bar:hover{ transform: scaleY(1.05); filter:brightness(1.08) }

/* responsive adjustments */
@media (max-width: 980px){ .chart-card{ height:340px } .metric-value{ font-size:24px } .title-text h1{ font-size:16px } }

/* subtle rim lighting for panels */
.el-card::after{ content:''; position:absolute; inset:0; pointer-events:none; border-radius:12px; box-shadow: inset 0 0 40px rgba(139,92,246,0.02) }

/* small utility */
.autostart-toggle{ display:inline-flex; align-items:center; gap:8px; padding:6px 10px; border-left:1px solid rgba(255,255,255,0.08) }
.autostart-label{ font-size:12px; color:var(--text-secondary) }

/* ensure charts and points pop */
:deep(.chartjs-render-monitor) { border-radius:8px }
:deep(.chartjs-tooltip) { background: rgba(3,6,12,0.85) }
</style>
