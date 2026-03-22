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
  Loading, Moon, Setting, Filter, Edit, Lightning,
  Odometer, Mute, Cpu as CpuIcon, Timer
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
const advancedVisible = ref(true)
const modeOptions = [
  { label: '自动', value: 'auto',        icon: 'Odometer' },
  { label: '静音', value: 'silent',      icon: 'Mute' },
  { label: '均衡', value: 'balanced',    icon: 'Timer' },
  { label: '性能', value: 'performance', icon: 'Cpu' }
]

// 图表引用
const leftFanChart  = ref(null)
const rightFanChart = ref(null)
const speedChart    = ref(null)
const tempChart     = ref(null)

let leftFanChartInstance  = null
let rightFanChartInstance = null
let speedChartInstance    = null
let tempChartInstance     = null

// 温度颜色
const getTempColor = (temp) => {
  if (!temp) return '#C0C4CC'
  if (temp > 85) return '#F56C6C'
  if (temp > 70) return '#E6A23C'
  return '#67C23A'
}

const getTempLevel = (temp) => {
  if (!temp) return 'badge-gray'
  if (temp > 85) return 'badge-red'
  if (temp > 70) return 'badge-orange'
  return 'badge-green'
}

const getTempLevelText = (temp) => {
  if (!temp) return '未知'
  if (temp > 85) return '过热'
  if (temp > 70) return '偏高'
  return '正常'
}

const refreshAutostartStatus = async () => {
  try {
    const enabled = await window.__TAURI__.core.invoke('plugin:autostart|is_enabled')
    autostartEnabled.value = !!enabled
    return enabled
  } catch (error) {
    console.error('❌ 检查开机自启状态失败:', error)
    ElMessage.error('无法检查开机自启: ' + error)
    return null
  }
}

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
      right_fan: buildCurveFromMode(rightFanChartInstance.data.labels, mode.value, k)
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
        }))
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
      }))
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

onMounted(async () => {
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
/* ===== 基础布局 ===== */
.app-container {
  padding: 14px;
  background: #f0f2f5;
  min-height: 100vh;
  transition: background 0.3s, color 0.3s;
}

/* 深色模式 */
.dark-mode {
  background: #1a1a2e;
  color: #e0e0e0;
}
.dark-mode :deep(.el-card) {
  background: #16213e;
  border-color: #0f3460;
  color: #e0e0e0;
}
.dark-mode .title-text h1,
.dark-mode .metric-value,
.dark-mode .chart-title { color: #e0e0e0; }
.dark-mode .metric-label,
.dark-mode .metric-unit,
.dark-mode .chart-subtitle { color: #aaa; }

/* ===== 导航栏 ===== */
.navbar { margin-bottom: 12px; }
.navbar-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.app-title { display: flex; align-items: center; gap: 12px; }
.title-icon { color: #409EFF; }
.title-text h1 { font-size: 18px; font-weight: 700; color: #303133; margin: 0; }
.title-text p  { font-size: 11px; color: #909399; margin: 2px 0 0 0; }
.status-bar { display: flex; gap: 8px; align-items: center; }

/* 运行中的 tag 脉冲效果 */
.pulse-tag { animation: pulse 1.8s ease-in-out infinite; }
@keyframes pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(103,194,58,0.4); }
  50%       { box-shadow: 0 0 0 6px rgba(103,194,58,0); }
}
.spin-icon { animation: spin 1.5s linear infinite; margin-right: 4px; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

.autostart-toggle {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  border-left: 1px solid #ebeef5;
}
.autostart-label { font-size: 12px; color: #606266; }

/* ===== 过温警告 ===== */
.overheat-alert {
  margin-bottom: 10px;
  border-radius: 8px;
  animation: shake 0.4s ease-in-out;
}
@keyframes shake {
  0%, 100% { transform: translateX(0); }
  25%       { transform: translateX(-4px); }
  75%       { transform: translateX(4px); }
}

/* ===== 指标卡 ===== */
.metrics-grid { margin-bottom: 12px; }
.metric-card { padding: 0; }
:deep(.metric-card .el-card__body) { padding: 14px 16px; }

.metric-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 10px;
  color: #909399;
}
.metric-label { font-size: 11px; font-weight: 600; text-transform: uppercase; flex: 1; }

/* 状态小徽章 */
.metric-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 8px;
  font-weight: 600;
}
.badge-gray   { background: #f4f4f5; color: #909399; }
.badge-green  { background: #f0f9eb; color: #67C23A; }
.badge-orange { background: #fdf6ec; color: #E6A23C; }
.badge-red    { background: #fef0f0; color: #F56C6C; }
.badge-blue   { background: #ecf5ff; color: #409EFF; }

.metric-value-container { display: flex; align-items: baseline; gap: 4px; }
.metric-value {
  font-size: 30px;
  font-weight: 700;
  color: #303133;
  line-height: 1;
  font-variant-numeric: tabular-nums;
  transition: color 0.5s;
}
.fan-value { color: #303133 !important; }
.metric-unit { font-size: 14px; color: #909399; }

.metric-progress { margin: 8px 0 6px; }

/* Sparkline 迷你历史 */
.metric-trend {
  display: flex;
  align-items: flex-end;
  gap: 2px;
  height: 20px;
  margin-top: 4px;
}
.spark-bar {
  flex: 1;
  min-height: 2px;
  border-radius: 1px;
  transition: height 0.3s, background 0.3s;
  opacity: 0.7;
}

/* ===== 快捷模式卡 ===== */
.quick-card { margin-bottom: 12px; }
.quick-row {
  display: flex;
  gap: 16px;
  align-items: center;
  flex-wrap: wrap;
}
.quick-block {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-width: 200px;
}
.quick-block.stretch { flex: 1; min-width: 260px; }
.quick-label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  color: #606266;
  gap: 4px;
}
.quick-actions { display: flex; gap: 8px; align-items: center; flex-shrink: 0; }

/* ===== 图表区 ===== */
.charts-container { margin-bottom: 12px; }

.chart-card { height: calc(50vh - 90px); min-height: 220px; }
:deep(.chart-card .el-card__body) { height: calc(100% - 56px); padding: 10px 14px; }

.chart-header {
  display: flex;
  align-items: center;
  gap: 8px;
}
.chart-title { font-size: 14px; font-weight: 600; color: #303133; }
.chart-subtitle { font-size: 11px; color: #909399; margin: 1px 0 0 0; }

.chart-wrapper { height: 100%; position: relative; }
.chart-wrapper canvas { width: 100% !important; height: 100% !important; }

/* ===== 控制按钮 ===== */
.controls {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 10px;
  padding: 4px 0 8px;
}
.main-control-btn { min-width: 140px; font-weight: 600; }
</style>
