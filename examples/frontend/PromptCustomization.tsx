import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui/tabs';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Switch } from '@/components/ui/switch';
import { Separator } from '@/components/ui/separator';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Settings,
  Play,
  Pause,
  BarChart3,
  FileText,
  Zap,
  TestTube,
  Brain,
  Target,
  Clock,
  TrendingUp,
  AlertTriangle,
  CheckCircle,
  MoreHorizontal,
} from 'lucide-react';

// 类型定义
interface PromptServiceStatus {
  is_initialized: boolean;
  active_sessions: number;
  total_prompts_built: number;
  cache_hit_rate: number;
  avg_build_time: number;
}

interface PromptSession {
  id: string;
  agent_profile?: string;
  domain_template?: string;
  created_at: string;
  last_activity: string;
  prompt_count: number;
  avg_build_time: number;
}

interface PromptConfig {
  id: string;
  name: string;
  description: string;
  agent_profile?: string;
  domain_template?: string;
  temperature: number;
  max_tokens: number;
  created_at: string;
  updated_at: string;
}

interface ABTest {
  id: string;
  name: string;
  description: string;
  status: 'draft' | 'running' | 'paused' | 'completed';
  variants: TestVariant[];
  traffic_allocation: number;
  start_date?: string;
  end_date?: string;
  sample_size: number;
  confidence: number;
  winning_variant?: string;
}

interface TestVariant {
  id: string;
  name: string;
  description: string;
  is_control: boolean;
  traffic_percentage: number;
  conversion_rate: number;
  sample_size: number;
}

interface PerformanceMetrics {
  success_rate: number;
  avg_execution_time: number;
  user_satisfaction: number;
  error_rate: number;
  cache_hit_rate: number;
  total_requests: number;
}

const PromptCustomization: React.FC = () => {
  // 状态管理
  const [serviceStatus, setServiceStatus] = useState<PromptServiceStatus | null>(null);
  const [sessions, setSessions] = useState<PromptSession[]>([]);
  const [configs, setConfigs] = useState<PromptConfig[]>([]);
  const [abTests, setABTests] = useState<ABTest[]>([]);
  const [metrics, setMetrics] = useState<PerformanceMetrics | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 表单状态
  const [newSessionForm, setNewSessionForm] = useState({
    sessionId: '',
    agentProfile: '',
    domainTemplate: '',
  });

  const [promptBuildForm, setPromptBuildForm] = useState({
    sessionId: '',
    buildType: 'planner',
    userQuery: '',
    targetInfo: '',
  });

  const [abTestForm, setABTestForm] = useState({
    name: '',
    description: '',
    variantA: '',
    variantB: '',
    trafficSplit: 50,
  });

  // 初始化
  useEffect(() => {
    initializeService();
    loadData();
  }, []);

  const initializeService = async () => {
    try {
      setLoading(true);
      await invoke('initialize_prompt_service', {
        config: {
          config_dir: './config/prompts',
          template_dir: './templates/prompts',
          cache_size: 1000,
          enable_hot_reload: true,
          enable_ab_testing: true,
          enable_auto_optimization: true,
        },
      });
      await loadServiceStatus();
    } catch (err) {
      setError(`初始化服务失败: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const loadData = async () => {
    try {
      await Promise.all([
        loadServiceStatus(),
        loadSessions(),
        loadConfigs(),
        loadABTests(),
        loadMetrics(),
      ]);
    } catch (err) {
      setError(`加载数据失败: ${err}`);
    }
  };

  const loadServiceStatus = async () => {
    const status = await invoke<PromptServiceStatus>('get_prompt_service_status');
    setServiceStatus(status);
  };

  const loadSessions = async () => {
    // 这里应该调用实际的API
    setSessions([
      {
        id: 'session_1',
        agent_profile: 'security_analyst',
        domain_template: 'web_security',
        created_at: '2024-01-15T10:00:00Z',
        last_activity: '2024-01-15T10:30:00Z',
        prompt_count: 15,
        avg_build_time: 250,
      },
      {
        id: 'session_2',
        agent_profile: 'penetration_tester',
        domain_template: 'network_security',
        created_at: '2024-01-15T11:00:00Z',
        last_activity: '2024-01-15T11:45:00Z',
        prompt_count: 8,
        avg_build_time: 180,
      },
    ]);
  };

  const loadConfigs = async () => {
    const configs = await invoke<PromptConfig[]>('get_prompt_configs');
    setConfigs(configs);
  };

  const loadABTests = async () => {
    // 模拟数据
    setABTests([
      {
        id: 'test_1',
        name: '规划器模板优化测试',
        description: '测试新的规划器模板是否能提高成功率',
        status: 'running',
        variants: [
          {
            id: 'variant_a',
            name: '原始模板',
            description: '当前使用的模板',
            is_control: true,
            traffic_percentage: 50,
            conversion_rate: 0.85,
            sample_size: 1250,
          },
          {
            id: 'variant_b',
            name: '优化模板',
            description: '优化后的模板',
            is_control: false,
            traffic_percentage: 50,
            conversion_rate: 0.92,
            sample_size: 1180,
          },
        ],
        traffic_allocation: 0.5,
        start_date: '2024-01-10T00:00:00Z',
        sample_size: 2430,
        confidence: 0.95,
        winning_variant: 'variant_b',
      },
    ]);
  };

  const loadMetrics = async () => {
    setMetrics({
      success_rate: 0.89,
      avg_execution_time: 1250,
      user_satisfaction: 4.2,
      error_rate: 0.03,
      cache_hit_rate: 0.75,
      total_requests: 15420,
    });
  };

  const createSession = async () => {
    try {
      setLoading(true);
      await invoke('create_prompt_session', {
        sessionId: newSessionForm.sessionId,
        agentProfile: newSessionForm.agentProfile || null,
        domainTemplate: newSessionForm.domainTemplate || null,
      });
      await loadSessions();
      setNewSessionForm({ sessionId: '', agentProfile: '', domainTemplate: '' });
    } catch (err) {
      setError(`创建会话失败: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const buildPrompt = async () => {
    try {
      setLoading(true);
      const response = await invoke('build_prompt', {
        sessionId: promptBuildForm.sessionId,
        request: {
          build_type: promptBuildForm.buildType,
          context: {
            user_query: promptBuildForm.userQuery,
            target_info: promptBuildForm.targetInfo ? JSON.parse(promptBuildForm.targetInfo) : null,
            available_tools: null,
            execution_context: null,
            history: null,
            custom_variables: {},
          },
        },
      });
      console.log('Generated prompt:', response);
      // 这里可以显示生成的prompt
    } catch (err) {
      setError(`构建Prompt失败: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const createABTest = async () => {
    try {
      setLoading(true);
      const test = {
        id: `test_${Date.now()}`,
        name: abTestForm.name,
        description: abTestForm.description,
        variants: [
          {
            id: 'variant_a',
            name: '变体A',
            description: abTestForm.variantA,
            is_control: true,
            template_config: {},
          },
          {
            id: 'variant_b',
            name: '变体B',
            description: abTestForm.variantB,
            is_control: false,
            template_config: {},
          },
        ],
        traffic_allocation: abTestForm.trafficSplit / 100,
        evaluation_metrics: ['success_rate', 'execution_time'],
        conditions: {
          min_sample_size: 1000,
          max_duration: 604800, // 7天
          confidence_level: 0.95,
        },
      };
      
      await invoke('create_ab_test', { test });
      await loadABTests();
      setABTestForm({ name: '', description: '', variantA: '', variantB: '', trafficSplit: 50 });
    } catch (err) {
      setError(`创建A/B测试失败: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'running': return 'bg-green-500';
      case 'paused': return 'bg-yellow-500';
      case 'completed': return 'bg-blue-500';
      default: return 'bg-gray-500';
    }
  };

  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${(ms / 60000).toFixed(1)}m`;
  };

  const formatPercentage = (value: number) => {
    return `${(value * 100).toFixed(1)}%`;
  };

  return (
    <div className="container mx-auto p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">Prompt 高级定制</h1>
          <p className="text-muted-foreground mt-2">
            管理和优化 AI 代理的 Prompt 配置
          </p>
        </div>
        <div className="flex items-center space-x-2">
          {serviceStatus?.is_initialized ? (
            <Badge variant="outline" className="bg-green-50 text-green-700 border-green-200">
              <CheckCircle className="w-3 h-3 mr-1" />
              服务已启动
            </Badge>
          ) : (
            <Badge variant="outline" className="bg-red-50 text-red-700 border-red-200">
              <AlertTriangle className="w-3 h-3 mr-1" />
              服务未启动
            </Badge>
          )}
        </div>
      </div>

      {error && (
        <Alert variant="destructive">
          <AlertTriangle className="h-4 w-4" />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {/* 服务状态概览 */}
      {serviceStatus && (
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          <Card>
            <CardContent className="p-4">
              <div className="flex items-center space-x-2">
                <Target className="h-4 w-4 text-blue-500" />
                <div>
                  <p className="text-sm font-medium">活跃会话</p>
                  <p className="text-2xl font-bold">{serviceStatus.active_sessions}</p>
                </div>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <div className="flex items-center space-x-2">
                <FileText className="h-4 w-4 text-green-500" />
                <div>
                  <p className="text-sm font-medium">总Prompt数</p>
                  <p className="text-2xl font-bold">{serviceStatus.total_prompts_built}</p>
                </div>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <div className="flex items-center space-x-2">
                <Zap className="h-4 w-4 text-yellow-500" />
                <div>
                  <p className="text-sm font-medium">缓存命中率</p>
                  <p className="text-2xl font-bold">{formatPercentage(serviceStatus.cache_hit_rate)}</p>
                </div>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <div className="flex items-center space-x-2">
                <Clock className="h-4 w-4 text-purple-500" />
                <div>
                  <p className="text-sm font-medium">平均构建时间</p>
                  <p className="text-2xl font-bold">{formatDuration(serviceStatus.avg_build_time)}</p>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      <Tabs defaultValue="sessions" className="space-y-4">
        <TabsList>
          <TabsTrigger value="sessions">会话管理</TabsTrigger>
          <TabsTrigger value="configs">配置管理</TabsTrigger>
          <TabsTrigger value="templates">模板管理</TabsTrigger>
          <TabsTrigger value="ab-tests">A/B测试</TabsTrigger>
          <TabsTrigger value="optimization">自动优化</TabsTrigger>
          <TabsTrigger value="metrics">性能监控</TabsTrigger>
        </TabsList>

        {/* 会话管理 */}
        <TabsContent value="sessions" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>创建新会话</CardTitle>
              <CardDescription>
                创建一个新的 Prompt 会话来开始构建定制化的 Prompt
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div>
                  <Label htmlFor="sessionId">会话ID</Label>
                  <Input
                    id="sessionId"
                    value={newSessionForm.sessionId}
                    onChange={(e) => setNewSessionForm(prev => ({ ...prev, sessionId: e.target.value }))}
                    placeholder="输入会话ID"
                  />
                </div>
                <div>
                  <Label htmlFor="agentProfile">代理配置</Label>
                  <Select
                    value={newSessionForm.agentProfile}
                    onValueChange={(value) => setNewSessionForm(prev => ({ ...prev, agentProfile: value }))}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="选择代理配置" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="security_analyst">安全分析师</SelectItem>
                      <SelectItem value="penetration_tester">渗透测试专家</SelectItem>
                      <SelectItem value="compliance_auditor">合规审计员</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label htmlFor="domainTemplate">领域模板</Label>
                  <Select
                    value={newSessionForm.domainTemplate}
                    onValueChange={(value) => setNewSessionForm(prev => ({ ...prev, domainTemplate: value }))}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="选择领域模板" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="web_security">Web安全测试</SelectItem>
                      <SelectItem value="network_security">网络安全测试</SelectItem>
                      <SelectItem value="mobile_security">移动应用安全</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <Button onClick={createSession} disabled={loading || !newSessionForm.sessionId}>
                创建会话
              </Button>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>活跃会话</CardTitle>
              <CardDescription>
                当前系统中的所有活跃 Prompt 会话
              </CardDescription>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>会话ID</TableHead>
                    <TableHead>代理配置</TableHead>
                    <TableHead>领域模板</TableHead>
                    <TableHead>Prompt数量</TableHead>
                    <TableHead>平均构建时间</TableHead>
                    <TableHead>最后活动</TableHead>
                    <TableHead>操作</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {sessions.map((session) => (
                    <TableRow key={session.id}>
                      <TableCell className="font-medium">{session.id}</TableCell>
                      <TableCell>
                        {session.agent_profile && (
                          <Badge variant="outline">{session.agent_profile}</Badge>
                        )}
                      </TableCell>
                      <TableCell>
                        {session.domain_template && (
                          <Badge variant="outline">{session.domain_template}</Badge>
                        )}
                      </TableCell>
                      <TableCell>{session.prompt_count}</TableCell>
                      <TableCell>{formatDuration(session.avg_build_time)}</TableCell>
                      <TableCell>{new Date(session.last_activity).toLocaleString()}</TableCell>
                      <TableCell>
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="sm">
                              <MoreHorizontal className="h-4 w-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent>
                            <DropdownMenuItem>查看详情</DropdownMenuItem>
                            <DropdownMenuItem>导出数据</DropdownMenuItem>
                            <DropdownMenuItem className="text-red-600">关闭会话</DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>

          {/* Prompt构建测试 */}
          <Card>
            <CardHeader>
              <CardTitle>Prompt构建测试</CardTitle>
              <CardDescription>
                测试 Prompt 构建功能
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="buildSessionId">会话ID</Label>
                  <Select
                    value={promptBuildForm.sessionId}
                    onValueChange={(value) => setPromptBuildForm(prev => ({ ...prev, sessionId: value }))}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="选择会话" />
                    </SelectTrigger>
                    <SelectContent>
                      {sessions.map((session) => (
                        <SelectItem key={session.id} value={session.id}>
                          {session.id}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label htmlFor="buildType">构建类型</Label>
                  <Select
                    value={promptBuildForm.buildType}
                    onValueChange={(value) => setPromptBuildForm(prev => ({ ...prev, buildType: value }))}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="planner">规划器</SelectItem>
                      <SelectItem value="executor">执行器</SelectItem>
                      <SelectItem value="replanner">重规划器</SelectItem>
                      <SelectItem value="report_generator">报告生成器</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
              <div>
                <Label htmlFor="userQuery">用户查询</Label>
                <Textarea
                  id="userQuery"
                  value={promptBuildForm.userQuery}
                  onChange={(e) => setPromptBuildForm(prev => ({ ...prev, userQuery: e.target.value }))}
                  placeholder="输入用户查询内容"
                  rows={3}
                />
              </div>
              <div>
                <Label htmlFor="targetInfo">目标信息 (JSON)</Label>
                <Textarea
                  id="targetInfo"
                  value={promptBuildForm.targetInfo}
                  onChange={(e) => setPromptBuildForm(prev => ({ ...prev, targetInfo: e.target.value }))}
                  placeholder='{"url": "https://example.com", "type": "web_application"}'
                  rows={3}
                />
              </div>
              <Button onClick={buildPrompt} disabled={loading || !promptBuildForm.sessionId}>
                <Brain className="w-4 h-4 mr-2" />
                构建Prompt
              </Button>
            </CardContent>
          </Card>
        </TabsContent>

        {/* A/B测试 */}
        <TabsContent value="ab-tests" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>创建A/B测试</CardTitle>
              <CardDescription>
                创建新的A/B测试来优化Prompt性能
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="testName">测试名称</Label>
                  <Input
                    id="testName"
                    value={abTestForm.name}
                    onChange={(e) => setABTestForm(prev => ({ ...prev, name: e.target.value }))}
                    placeholder="输入测试名称"
                  />
                </div>
                <div>
                  <Label htmlFor="trafficSplit">流量分配 (%)</Label>
                  <Input
                    id="trafficSplit"
                    type="number"
                    min="0"
                    max="100"
                    value={abTestForm.trafficSplit}
                    onChange={(e) => setABTestForm(prev => ({ ...prev, trafficSplit: parseInt(e.target.value) }))}
                  />
                </div>
              </div>
              <div>
                <Label htmlFor="testDescription">测试描述</Label>
                <Textarea
                  id="testDescription"
                  value={abTestForm.description}
                  onChange={(e) => setABTestForm(prev => ({ ...prev, description: e.target.value }))}
                  placeholder="描述测试目标和预期结果"
                  rows={2}
                />
              </div>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="variantA">变体A (控制组)</Label>
                  <Textarea
                    id="variantA"
                    value={abTestForm.variantA}
                    onChange={(e) => setABTestForm(prev => ({ ...prev, variantA: e.target.value }))}
                    placeholder="描述控制组配置"
                    rows={3}
                  />
                </div>
                <div>
                  <Label htmlFor="variantB">变体B (实验组)</Label>
                  <Textarea
                    id="variantB"
                    value={abTestForm.variantB}
                    onChange={(e) => setABTestForm(prev => ({ ...prev, variantB: e.target.value }))}
                    placeholder="描述实验组配置"
                    rows={3}
                  />
                </div>
              </div>
              <Button onClick={createABTest} disabled={loading || !abTestForm.name}>
                <TestTube className="w-4 h-4 mr-2" />
                创建测试
              </Button>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>A/B测试列表</CardTitle>
              <CardDescription>
                当前系统中的所有A/B测试
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="space-y-4">
                {abTests.map((test) => (
                  <Card key={test.id} className="border-l-4 border-l-blue-500">
                    <CardContent className="p-4">
                      <div className="flex items-start justify-between">
                        <div className="space-y-2">
                          <div className="flex items-center space-x-2">
                            <h3 className="font-semibold">{test.name}</h3>
                            <Badge className={getStatusColor(test.status)}>
                              {test.status === 'running' ? '运行中' : 
                               test.status === 'paused' ? '已暂停' : 
                               test.status === 'completed' ? '已完成' : '草稿'}
                            </Badge>
                          </div>
                          <p className="text-sm text-muted-foreground">{test.description}</p>
                          <div className="flex items-center space-x-4 text-sm">
                            <span>样本量: {test.sample_size}</span>
                            <span>置信度: {formatPercentage(test.confidence)}</span>
                            {test.winning_variant && (
                              <span className="text-green-600">
                                获胜变体: {test.variants.find(v => v.id === test.winning_variant)?.name}
                              </span>
                            )}
                          </div>
                        </div>
                        <div className="flex space-x-2">
                          {test.status === 'running' ? (
                            <Button size="sm" variant="outline">
                              <Pause className="w-4 h-4 mr-1" />
                              暂停
                            </Button>
                          ) : (
                            <Button size="sm" variant="outline">
                              <Play className="w-4 h-4 mr-1" />
                              开始
                            </Button>
                          )}
                          <Button size="sm" variant="outline">
                            <BarChart3 className="w-4 h-4 mr-1" />
                            查看结果
                          </Button>
                        </div>
                      </div>
                      
                      <Separator className="my-3" />
                      
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        {test.variants.map((variant) => (
                          <div key={variant.id} className="space-y-2">
                            <div className="flex items-center justify-between">
                              <span className="font-medium">{variant.name}</span>
                              {variant.is_control && <Badge variant="outline">控制组</Badge>}
                            </div>
                            <div className="space-y-1">
                              <div className="flex justify-between text-sm">
                                <span>转化率:</span>
                                <span className="font-medium">{formatPercentage(variant.conversion_rate)}</span>
                              </div>
                              <div className="flex justify-between text-sm">
                                <span>样本量:</span>
                                <span>{variant.sample_size}</span>
                              </div>
                              <div className="flex justify-between text-sm">
                                <span>流量分配:</span>
                                <span>{formatPercentage(variant.traffic_percentage / 100)}</span>
                              </div>
                            </div>
                            <Progress value={variant.conversion_rate * 100} className="h-2" />
                          </div>
                        ))}
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* 性能监控 */}
        <TabsContent value="metrics" className="space-y-4">
          {metrics && (
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <Card>
                <CardContent className="p-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium text-muted-foreground">成功率</p>
                      <p className="text-2xl font-bold">{formatPercentage(metrics.success_rate)}</p>
                    </div>
                    <TrendingUp className="h-8 w-8 text-green-500" />
                  </div>
                  <Progress value={metrics.success_rate * 100} className="mt-2" />
                </CardContent>
              </Card>
              
              <Card>
                <CardContent className="p-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium text-muted-foreground">平均执行时间</p>
                      <p className="text-2xl font-bold">{formatDuration(metrics.avg_execution_time)}</p>
                    </div>
                    <Clock className="h-8 w-8 text-blue-500" />
                  </div>
                </CardContent>
              </Card>
              
              <Card>
                <CardContent className="p-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium text-muted-foreground">用户满意度</p>
                      <p className="text-2xl font-bold">{metrics.user_satisfaction.toFixed(1)}/5.0</p>
                    </div>
                    <Target className="h-8 w-8 text-purple-500" />
                  </div>
                  <Progress value={(metrics.user_satisfaction / 5) * 100} className="mt-2" />
                </CardContent>
              </Card>
              
              <Card>
                <CardContent className="p-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium text-muted-foreground">错误率</p>
                      <p className="text-2xl font-bold">{formatPercentage(metrics.error_rate)}</p>
                    </div>
                    <AlertTriangle className="h-8 w-8 text-red-500" />
                  </div>
                  <Progress value={metrics.error_rate * 100} className="mt-2" />
                </CardContent>
              </Card>
              
              <Card>
                <CardContent className="p-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium text-muted-foreground">缓存命中率</p>
                      <p className="text-2xl font-bold">{formatPercentage(metrics.cache_hit_rate)}</p>
                    </div>
                    <Zap className="h-8 w-8 text-yellow-500" />
                  </div>
                  <Progress value={metrics.cache_hit_rate * 100} className="mt-2" />
                </CardContent>
              </Card>
              
              <Card>
                <CardContent className="p-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <p className="text-sm font-medium text-muted-foreground">总请求数</p>
                      <p className="text-2xl font-bold">{metrics.total_requests.toLocaleString()}</p>
                    </div>
                    <BarChart3 className="h-8 w-8 text-indigo-500" />
                  </div>
                </CardContent>
              </Card>
            </div>
          )}
        </TabsContent>

        {/* 其他标签页的占位符 */}
        <TabsContent value="configs">
          <Card>
            <CardHeader>
              <CardTitle>配置管理</CardTitle>
              <CardDescription>管理Prompt配置</CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-muted-foreground">配置管理功能开发中...</p>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="templates">
          <Card>
            <CardHeader>
              <CardTitle>模板管理</CardTitle>
              <CardDescription>管理Prompt模板</CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-muted-foreground">模板管理功能开发中...</p>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="optimization">
          <Card>
            <CardHeader>
              <CardTitle>自动优化</CardTitle>
              <CardDescription>自动优化Prompt性能</CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-muted-foreground">自动优化功能开发中...</p>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
};

export default PromptCustomization;