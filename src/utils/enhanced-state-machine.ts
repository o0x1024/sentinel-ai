import {
  DetailedExecutionStatus,
  StateTransition,
  StateMachineConfig,
  EnhancedExecutionContext,
  StateHistoryEntry,
  FlowAdjustment,
  StateTransitionEvent,
  RealTimeUpdate,
  ExecutionStrategy,
  UserIntervention,
  FlowchartNode,
  FlowchartConnection
} from '../types/enhanced-execution'

// 增强的状态机管理器
export class EnhancedStateMachine {
  private config: StateMachineConfig
  private context: EnhancedExecutionContext
  private listeners: Map<string, (event: StateTransitionEvent) => void> = new Map()
  private realTimeListeners: Map<string, (update: RealTimeUpdate) => void> = new Map()
  private adjustmentHistory: FlowAdjustment[] = []

  constructor(config: StateMachineConfig, context: EnhancedExecutionContext) {
    this.config = config
    this.context = context
    this.initializeStateMachine()
  }

  private initializeStateMachine() {
    // 初始化允许的状态转换映射
    this.config.allowedTransitions = new Map()
    
    // 定义状态转换规则
    const transitions: [DetailedExecutionStatus, DetailedExecutionStatus[]][] = [
      // 初始化阶段
      [DetailedExecutionStatus.INITIALIZED, [
        DetailedExecutionStatus.PLANNING_STARTED,
        DetailedExecutionStatus.CANCELLED
      ]],
      
      // 规划阶段转换
      [DetailedExecutionStatus.PLANNING_STARTED, [
        DetailedExecutionStatus.ANALYZING_INTENT,
        DetailedExecutionStatus.PLANNING_FAILED
      ]],
      [DetailedExecutionStatus.ANALYZING_INTENT, [
        DetailedExecutionStatus.RETRIEVING_MEMORY,
        DetailedExecutionStatus.PLANNING_FAILED
      ]],
      [DetailedExecutionStatus.RETRIEVING_MEMORY, [
        DetailedExecutionStatus.GENERATING_PLAN,
        DetailedExecutionStatus.PLANNING_FAILED
      ]],
      [DetailedExecutionStatus.GENERATING_PLAN, [
        DetailedExecutionStatus.OPTIMIZING_PLAN,
        DetailedExecutionStatus.PLANNING_FAILED
      ]],
      [DetailedExecutionStatus.OPTIMIZING_PLAN, [
        DetailedExecutionStatus.VALIDATING_PLAN,
        DetailedExecutionStatus.PLANNING_FAILED
      ]],
      [DetailedExecutionStatus.VALIDATING_PLAN, [
        DetailedExecutionStatus.PLANNING_COMPLETED,
        DetailedExecutionStatus.PLANNING_FAILED,
        DetailedExecutionStatus.REQUIRES_INTERVENTION
      ]],
      [DetailedExecutionStatus.PLANNING_COMPLETED, [
        DetailedExecutionStatus.EXECUTION_STARTED
      ]],
      
      // 执行阶段转换
      [DetailedExecutionStatus.EXECUTION_STARTED, [
        DetailedExecutionStatus.PREPARING_CONTEXT,
        DetailedExecutionStatus.FAILED
      ]],
      [DetailedExecutionStatus.PREPARING_CONTEXT, [
        DetailedExecutionStatus.STEP_EXECUTING,
        DetailedExecutionStatus.FAILED
      ]],
      [DetailedExecutionStatus.STEP_EXECUTING, [
        DetailedExecutionStatus.TOOL_CALLING,
        DetailedExecutionStatus.STEP_COMPLETED,
        DetailedExecutionStatus.STEP_FAILED,
        DetailedExecutionStatus.PAUSING,
        DetailedExecutionStatus.REQUIRES_INTERVENTION
      ]],
      [DetailedExecutionStatus.TOOL_CALLING, [
        DetailedExecutionStatus.WAITING_TOOL_RESPONSE,
        DetailedExecutionStatus.STEP_FAILED
      ]],
      [DetailedExecutionStatus.WAITING_TOOL_RESPONSE, [
        DetailedExecutionStatus.PROCESSING_RESULT,
        DetailedExecutionStatus.STEP_FAILED
      ]],
      [DetailedExecutionStatus.PROCESSING_RESULT, [
        DetailedExecutionStatus.STEP_COMPLETED,
        DetailedExecutionStatus.STEP_FAILED,
        DetailedExecutionStatus.REPLAN_TRIGGERED
      ]],
      [DetailedExecutionStatus.STEP_COMPLETED, [
        DetailedExecutionStatus.STEP_EXECUTING,
        DetailedExecutionStatus.MONITORING_STARTED,
        DetailedExecutionStatus.COMPLETING
      ]],
      [DetailedExecutionStatus.STEP_FAILED, [
        DetailedExecutionStatus.STEP_EXECUTING, // 重试
        DetailedExecutionStatus.REPLAN_TRIGGERED,
        DetailedExecutionStatus.FAILED,
        DetailedExecutionStatus.REQUIRES_INTERVENTION
      ]],
      
      // 监控阶段转换
      [DetailedExecutionStatus.MONITORING_STARTED, [
        DetailedExecutionStatus.CHECKING_PROGRESS,
        DetailedExecutionStatus.STEP_EXECUTING
      ]],
      [DetailedExecutionStatus.CHECKING_PROGRESS, [
        DetailedExecutionStatus.DETECTING_ANOMALY,
        DetailedExecutionStatus.EVALUATING_PERFORMANCE,
        DetailedExecutionStatus.STEP_EXECUTING
      ]],
      [DetailedExecutionStatus.DETECTING_ANOMALY, [
        DetailedExecutionStatus.REPLAN_TRIGGERED,
        DetailedExecutionStatus.STEP_EXECUTING,
        DetailedExecutionStatus.REQUIRES_INTERVENTION
      ]],
      [DetailedExecutionStatus.EVALUATING_PERFORMANCE, [
        DetailedExecutionStatus.STEP_EXECUTING,
        DetailedExecutionStatus.REPLAN_TRIGGERED
      ]],
      
      // 重规划阶段转换
      [DetailedExecutionStatus.REPLAN_TRIGGERED, [
        DetailedExecutionStatus.ANALYZING_FAILURE,
        DetailedExecutionStatus.GENERATING_ALTERNATIVE
      ]],
      [DetailedExecutionStatus.ANALYZING_FAILURE, [
        DetailedExecutionStatus.GENERATING_ALTERNATIVE,
        DetailedExecutionStatus.REPLAN_FAILED
      ]],
      [DetailedExecutionStatus.GENERATING_ALTERNATIVE, [
        DetailedExecutionStatus.UPDATING_PLAN,
        DetailedExecutionStatus.REPLAN_FAILED
      ]],
      [DetailedExecutionStatus.UPDATING_PLAN, [
        DetailedExecutionStatus.REPLAN_COMPLETED,
        DetailedExecutionStatus.REPLAN_FAILED
      ]],
      [DetailedExecutionStatus.REPLAN_COMPLETED, [
        DetailedExecutionStatus.STEP_EXECUTING
      ]],
      [DetailedExecutionStatus.REPLAN_FAILED, [
        DetailedExecutionStatus.FAILED,
        DetailedExecutionStatus.REQUIRES_INTERVENTION
      ]],
      
      // 暂停和恢复
      [DetailedExecutionStatus.PAUSING, [
        DetailedExecutionStatus.PAUSED
      ]],
      [DetailedExecutionStatus.PAUSED, [
        DetailedExecutionStatus.RESUMING,
        DetailedExecutionStatus.CANCELLING
      ]],
      [DetailedExecutionStatus.RESUMING, [
        DetailedExecutionStatus.STEP_EXECUTING
      ]],
      
      // 终止状态转换
      [DetailedExecutionStatus.COMPLETING, [
        DetailedExecutionStatus.COMPLETED
      ]],
      [DetailedExecutionStatus.CANCELLING, [
        DetailedExecutionStatus.CANCELLED
      ]],
      
      // 需要干预
      [DetailedExecutionStatus.REQUIRES_INTERVENTION, [
        DetailedExecutionStatus.WAITING_USER_INPUT
      ]],
      [DetailedExecutionStatus.WAITING_USER_INPUT, [
        DetailedExecutionStatus.STEP_EXECUTING,
        DetailedExecutionStatus.REPLAN_TRIGGERED,
        DetailedExecutionStatus.CANCELLED
      ]]
    ]
    
    transitions.forEach(([from, toStates]) => {
      this.config.allowedTransitions.set(from, toStates)
    })
  }

  // 状态转换
  async transitionTo(
    newState: DetailedExecutionStatus,
    metadata?: Record<string, any>,
    triggeredBy: 'system' | 'user' | 'external' = 'system'
  ): Promise<boolean> {
    const currentState = this.context.currentState
    
    // 检查转换是否允许
    if (!this.isTransitionAllowed(currentState, newState)) {
      console.warn(`Invalid state transition from ${currentState} to ${newState}`)
      return false
    }
    
    const startTime = Date.now()
    
    try {
      // 执行状态处理器
      const handler = this.config.stateHandlers.get(newState)
      if (handler) {
        await handler(this.context as any)
      }
      
      // 更新状态历史
      const historyEntry: StateHistoryEntry = {
        state: currentState,
        timestamp: new Date(startTime),
        duration: Date.now() - startTime,
        metadata,
        triggeredBy
      }
      
      this.context.stateHistory.push(historyEntry)
      this.context.previousState = currentState
      this.context.currentState = newState
      
      // 更新流程图
      this.updateFlowchartForStateChange(currentState, newState)
      
      // 发送状态转换事件
      const event: StateTransitionEvent = {
        sessionId: this.context.sessionId,
        from: currentState,
        to: newState,
        timestamp: new Date(),
        metadata,
        flowchartUpdate: {
          nodeUpdates: this.getNodeUpdatesForState(newState),
          connectionUpdates: this.getConnectionUpdatesForState(currentState, newState)
        }
      }
      
      this.emitStateTransitionEvent(event)
      
      // 发送实时更新
      this.emitRealTimeUpdate({
        type: 'state_change',
        sessionId: this.context.sessionId,
        data: { from: currentState, to: newState, metadata },
        timestamp: new Date()
      })
      
      return true
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error)
      console.error(`Error during state transition to ${newState}:`, error)
      
      // 如果转换失败，尝试转换到失败状态
      if (newState !== DetailedExecutionStatus.FAILED) {
        await this.transitionTo(DetailedExecutionStatus.FAILED, {
          error: errorMessage,
          originalTarget: newState
        })
      }
      
      return false
    }
  }

  // 检查状态转换是否允许
  private isTransitionAllowed(from: DetailedExecutionStatus, to: DetailedExecutionStatus): boolean {
    const allowedStates = this.config.allowedTransitions.get(from)
    return allowedStates?.includes(to) ?? false
  }

  // 动态调整流程
  async adjustFlow(adjustment: Omit<FlowAdjustment, 'id' | 'timestamp'>): Promise<boolean> {
    const flowAdjustment: FlowAdjustment = {
      ...adjustment,
      id: `adj_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      timestamp: new Date()
    }
    
    try {
      switch (adjustment.type) {
        case 'add_step':
          await this.addStep(adjustment.data)
          break
        case 'remove_step':
          await this.removeStep(adjustment.target)
          break
        case 'modify_step':
          await this.modifyStep(adjustment.target, adjustment.data)
          break
        case 'change_order':
          await this.changeStepOrder(adjustment.data)
          break
        case 'add_condition':
          await this.addCondition(adjustment.target, adjustment.data)
          break
        case 'modify_condition':
          await this.modifyCondition(adjustment.target, adjustment.data)
          break
        default:
          throw new Error(`Unknown adjustment type: ${adjustment.type}`)
      }
      
      this.adjustmentHistory.push(flowAdjustment)
      
      // 发送实时更新
      this.emitRealTimeUpdate({
        type: 'state_change',
        sessionId: this.context.sessionId,
        data: { adjustment: flowAdjustment },
        timestamp: new Date()
      })
      
      return true
    } catch (error) {
      console.error('Error adjusting flow:', error)
      return false
    }
  }

  // 添加步骤
  private async addStep(stepData: any): Promise<void> {
    // 实现添加步骤逻辑
    const newStep = {
      id: `step_${Date.now()}`,
      ...stepData
    }
    
    this.context.executionPlan.steps.push(newStep)
    
    // 更新流程图
    const newNode: FlowchartNode = {
      id: newStep.id,
      name: newStep.name,
      description: newStep.description,
      type: 'agent',
      status: DetailedExecutionStatus.INITIALIZED,
      position: { x: 0, y: 0 }, // 将由自动布局确定
      size: { width: 120, height: 80 },
      dependencies: stepData.dependencies || []
    }
    
    this.context.flowchartData.nodes.push(newNode)
    this.updateFlowchartConnections()
  }

  // 移除步骤
  private async removeStep(stepId: string): Promise<void> {
    // 移除执行计划中的步骤
    this.context.executionPlan.steps = this.context.executionPlan.steps.filter(
      step => step.id !== stepId
    )
    
    // 移除流程图中的节点
    this.context.flowchartData.nodes = this.context.flowchartData.nodes.filter(
      node => node.id !== stepId
    )
    
    // 移除相关连接
    this.context.flowchartData.connections = this.context.flowchartData.connections.filter(
      conn => conn.from !== stepId && conn.to !== stepId
    )
    
    this.updateFlowchartConnections()
  }

  // 修改步骤
  private async modifyStep(stepId: string, modifications: any): Promise<void> {
    const step = this.context.executionPlan.steps.find(s => s.id === stepId)
    if (step) {
      Object.assign(step, modifications)
    }
    
    const node = this.context.flowchartData.nodes.find(n => n.id === stepId)
    if (node) {
      Object.assign(node, modifications)
    }
  }

  // 改变步骤顺序
  private async changeStepOrder(orderData: { stepId: string; newIndex: number }[]): Promise<void> {
    const { stepId, newIndex } = orderData[0] // 简化实现
    const steps = this.context.executionPlan.steps
    const stepIndex = steps.findIndex(s => s.id === stepId)
    
    if (stepIndex !== -1) {
      const [step] = steps.splice(stepIndex, 1)
      steps.splice(newIndex, 0, step)
    }
    
    this.updateFlowchartConnections()
  }

  // 添加条件
  private async addCondition(targetId: string, conditionData: any): Promise<void> {
    // 实现添加条件逻辑
    const connection = this.context.flowchartData.connections.find(
      conn => conn.id === targetId
    )
    
    if (connection) {
      connection.condition = conditionData.condition
      connection.type = 'condition'
    }
  }

  // 修改条件
  private async modifyCondition(targetId: string, conditionData: any): Promise<void> {
    const connection = this.context.flowchartData.connections.find(
      conn => conn.id === targetId
    )
    
    if (connection) {
      Object.assign(connection, conditionData)
    }
  }

  // 更新流程图连接
  private updateFlowchartConnections(): void {
    const connections: FlowchartConnection[] = []
    
    this.context.flowchartData.nodes.forEach(node => {
      node.dependencies.forEach(depId => {
        const fromNode = this.context.flowchartData.nodes.find(n => n.id === depId)
        if (fromNode) {
          connections.push({
            id: `${depId}-${node.id}`,
            from: depId,
            to: node.id,
            type: 'sequence',
            status: 'inactive'
          })
        }
      })
    })
    
    this.context.flowchartData.connections = connections
  }

  // 更新流程图状态
  private updateFlowchartForStateChange(
    from: DetailedExecutionStatus,
    to: DetailedExecutionStatus
  ): void {
    // 根据当前步骤更新对应的节点状态
    if (this.context.currentStep) {
      const node = this.context.flowchartData.nodes.find(
        n => n.id === this.context.currentStep?.id
      )
      if (node) {
        node.status = to
      }
    }
    
    // 更新连接状态
    this.updateConnectionStates()
  }

  // 获取状态对应的节点更新
  private getNodeUpdatesForState(state: DetailedExecutionStatus): Partial<FlowchartNode>[] {
    const updates: Partial<FlowchartNode>[] = []
    
    if (this.context.currentStep) {
      updates.push({
        id: this.context.currentStep.id,
        status: state
      })
    }
    
    return updates
  }

  // 获取状态对应的连接更新
  private getConnectionUpdatesForState(
    from: DetailedExecutionStatus,
    to: DetailedExecutionStatus
  ): Partial<FlowchartConnection>[] {
    const updates: Partial<FlowchartConnection>[] = []
    
    // 根据状态转换更新连接状态
    if (this.context.currentStep) {
      const activeConnections = this.context.flowchartData.connections.filter(
        conn => conn.to === this.context.currentStep?.id
      )
      
      activeConnections.forEach(conn => {
        updates.push({
          id: conn.id,
          status: this.getConnectionStatusFromState(to)
        })
      })
    }
    
    return updates
  }

  // 根据状态获取连接状态
  private getConnectionStatusFromState(state: DetailedExecutionStatus): 'inactive' | 'active' | 'completed' | 'failed' {
    switch (state) {
      case DetailedExecutionStatus.STEP_EXECUTING:
      case DetailedExecutionStatus.TOOL_CALLING:
      case DetailedExecutionStatus.WAITING_TOOL_RESPONSE:
        return 'active'
      case DetailedExecutionStatus.STEP_COMPLETED:
        return 'completed'
      case DetailedExecutionStatus.STEP_FAILED:
      case DetailedExecutionStatus.FAILED:
        return 'failed'
      default:
        return 'inactive'
    }
  }

  // 更新连接状态
  private updateConnectionStates(): void {
    this.context.flowchartData.connections.forEach(connection => {
      const fromNode = this.context.flowchartData.nodes.find(n => n.id === connection.from)
      const toNode = this.context.flowchartData.nodes.find(n => n.id === connection.to)
      
      if (fromNode && toNode) {
        if (fromNode.status === DetailedExecutionStatus.COMPLETED && 
            toNode.status === DetailedExecutionStatus.STEP_EXECUTING) {
          connection.status = 'active'
        } else if (fromNode.status === DetailedExecutionStatus.COMPLETED && 
                   toNode.status === DetailedExecutionStatus.COMPLETED) {
          connection.status = 'completed'
        } else if (fromNode.status === DetailedExecutionStatus.FAILED || 
                   toNode.status === DetailedExecutionStatus.FAILED) {
          connection.status = 'failed'
        } else {
          connection.status = 'inactive'
        }
      }
    })
  }

  // 事件监听器管理
  addStateTransitionListener(id: string, listener: (event: StateTransitionEvent) => void): void {
    this.listeners.set(id, listener)
  }

  removeStateTransitionListener(id: string): void {
    this.listeners.delete(id)
  }

  addRealTimeListener(id: string, listener: (update: RealTimeUpdate) => void): void {
    this.realTimeListeners.set(id, listener)
  }

  removeRealTimeListener(id: string): void {
    this.realTimeListeners.delete(id)
  }

  // 发送事件
  private emitStateTransitionEvent(event: StateTransitionEvent): void {
    this.listeners.forEach(listener => {
      try {
        listener(event)
      } catch (error) {
        console.error('Error in state transition listener:', error)
      }
    })
  }

  private emitRealTimeUpdate(update: RealTimeUpdate): void {
    this.realTimeListeners.forEach(listener => {
      try {
        listener(update)
      } catch (error) {
        console.error('Error in real-time listener:', error)
      }
    })
  }

  // 获取当前状态信息
  getCurrentState(): DetailedExecutionStatus {
    return this.context.currentState
  }

  getStateHistory(): StateHistoryEntry[] {
    return [...this.context.stateHistory]
  }

  getAdjustmentHistory(): FlowAdjustment[] {
    return [...this.adjustmentHistory]
  }

  // 获取可能的下一个状态
  getPossibleNextStates(): DetailedExecutionStatus[] {
    return this.config.allowedTransitions.get(this.context.currentState) || []
  }

  // 检查是否需要用户干预
  requiresIntervention(): UserIntervention | null {
    return this.context.interventions.find(i => !i.resolvedAt) || null
  }

  // 解决用户干预
  async resolveIntervention(interventionId: string, response: string): Promise<boolean> {
    const intervention = this.context.interventions.find(i => i.id === interventionId)
    if (!intervention) {
      return false
    }
    
    intervention.response = response
    intervention.resolvedAt = new Date()
    
    // 根据干预类型决定下一个状态
    let nextState: DetailedExecutionStatus
    switch (intervention.type) {
      case 'approval':
        nextState = response === 'approved' ? 
          DetailedExecutionStatus.STEP_EXECUTING : 
          DetailedExecutionStatus.CANCELLED
        break
      case 'decision':
        nextState = DetailedExecutionStatus.STEP_EXECUTING
        break
      default:
        nextState = DetailedExecutionStatus.STEP_EXECUTING
    }
    
    return await this.transitionTo(nextState, {
      interventionResolved: interventionId,
      userResponse: response
    }, 'user')
  }
}

// 状态机工厂
export class StateMachineFactory {
  static create(
    strategy: ExecutionStrategy,
    context: EnhancedExecutionContext
  ): EnhancedStateMachine {
    const config: StateMachineConfig = {
      initialState: DetailedExecutionStatus.INITIALIZED,
      transitions: [],
      allowedTransitions: new Map(),
      stateHandlers: new Map()
    }
    
    // 根据策略配置状态处理器
    config.stateHandlers.set(DetailedExecutionStatus.PLANNING_STARTED, async (ctx) => {
      console.log('Starting planning phase')
      // 实现规划开始逻辑
    })
    
    config.stateHandlers.set(DetailedExecutionStatus.STEP_EXECUTING, async (ctx) => {
      const enhancedCtx = ctx as unknown as EnhancedExecutionContext
      console.log('Executing step:', enhancedCtx.currentStep?.name)
      // 实现步骤执行逻辑
    })
    
    config.stateHandlers.set(DetailedExecutionStatus.TOOL_CALLING, async (ctx) => {
      const enhancedCtx = ctx as unknown as EnhancedExecutionContext
      console.log('Calling tool for step:', enhancedCtx.currentStep?.name)
      // 实现工具调用逻辑
    })
    
    config.stateHandlers.set(DetailedExecutionStatus.REPLAN_TRIGGERED, async (ctx) => {
      console.log('Replanning triggered')
      // 实现重规划逻辑
    })
    
    return new EnhancedStateMachine(config, context)
  }
}