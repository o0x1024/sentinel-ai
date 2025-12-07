/**
 * 架构消息处理器工厂
 *
 * 根据不同的架构类型返回对应的处理器
 * 扩展设计：未来可以添加其他架构的处理器（ReWOO、LLMCompiler 等）
 */

import type { ArchitectureType, OrderedMessageChunk } from '../../types/ordered-chat'
import type { ChatMessage } from '../../types/chat'
import { ReActMessageProcessor } from './ReActMessageProcessor'
import { VisionExplorerMessageProcessor } from './VisionExplorerMessageProcessor'

/**
 * 架构消息处理器的通用接口
 */
export interface IArchitectureMessageProcessor {
  /**
   * 从消息构建架构特定的步骤/数据显示
   */
  buildDisplayData(message: ChatMessage): any

  /**
   * 从消息块数组中提取架构特定的步骤
   */
  extractStepsFromChunks(chunks: OrderedMessageChunk[]): any

  /**
   * 检查是否应该折叠特定的数据块
   */
  shouldCollapse(data: any): boolean

  /**
   * 格式化特定类型的数据
   */
  formatData(data: any): string
}

/**
 * 架构处理器工厂
 */
export class ArchitectureProcessorFactory {
  /**
   * 根据架构类型获取对应的处理器实例
   *
   * @param architectureType - 架构类型
   * @returns 对应的处理器，如果架构未支持则返回 null
   */
  static getProcessor(architectureType?: ArchitectureType): IArchitectureMessageProcessor | null {
    switch (architectureType) {
      case 'ReAct':
        return new ReActProcessorAdapter()
      case 'VisionExplorer':
        return new VisionExplorerProcessorAdapter()
      // ReWOO、LLMCompiler、PlanAndExecute 已内嵌到 ReAct
      case 'ReWOO':
      case 'LLMCompiler':
      case 'PlanAndExecute':
      default:
        return null
    }
  }

  /**
   * 检查是否为架构特定的消息
   *
   * @param message - 消息对象
   * @returns 消息是否有特定的架构类型
   */
  static hasArchitecture(message: ChatMessage): boolean {
    return !!message.architectureType && message.architectureType !== 'Unknown'
  }
}

/**
 * ReAct 处理器适配器
 * 将 ReActMessageProcessor 适配为通用接口
 */
class ReActProcessorAdapter implements IArchitectureMessageProcessor {
  buildDisplayData(message: ChatMessage): any {
    return ReActMessageProcessor.buildReActStepsFromMessage(message)
  }

  extractStepsFromChunks(chunks: OrderedMessageChunk[]): any {
    return ReActMessageProcessor.extractStepsFromChunks(chunks)
  }

  shouldCollapse(data: any): boolean {
    // ReAct 中 data 是 action 对象
    return ReActMessageProcessor.shouldCollapseToolCall(data)
  }

  formatData(data: any): string {
    return ReActMessageProcessor.formatJson(data)
  }
}

/**
 * VisionExplorer 处理器适配器
 * 将 VisionExplorerMessageProcessor 适配为通用接口
 */
class VisionExplorerProcessorAdapter implements IArchitectureMessageProcessor {
  buildDisplayData(message: ChatMessage): any {
    return VisionExplorerMessageProcessor.buildIterationsFromMessage(message)
  }

  extractStepsFromChunks(chunks: OrderedMessageChunk[]): any {
    return VisionExplorerMessageProcessor.extractIterationsFromChunks(chunks)
  }

  shouldCollapse(data: any): boolean {
    // VisionExplorer 迭代默认不折叠
    return false
  }

  formatData(data: any): string {
    return JSON.stringify(data, null, 2)
  }
}
