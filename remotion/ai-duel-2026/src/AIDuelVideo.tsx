import React from 'react';
import {AbsoluteFill, Easing, interpolate, spring, useCurrentFrame, useVideoConfig} from 'remotion';

type Scene = {
  headline: string;
  subline: string;
  bullets: string[];
  accent: string;
};

const scenes: Scene[] = [
  {
    headline: '中门对狙：AI 春晚夜',
    subline: 'Claude Opus 4.6 与 GPT-5.3 Codex 同时发布',
    bullets: [
      '同一天，两家头部公司放出旗舰更新',
      '核心议题：Agent 化、编程能力、真实生产力',
      '这不是参数竞赛，而是软件范式转向',
    ],
    accent: '#ff6b35',
  },
  {
    headline: 'Claude Opus 4.6：长上下文与流体智力',
    subline: '关键词：1M 上下文、128K 输出、Context Compaction',
    bullets: [
      'ARC-AGI-2 达到 68.8%，推理与模式识别显著增强',
      '1M 上下文下抗 context rot 表现更稳定',
      'Agent Teams 强化多代理协作与相互质疑',
    ],
    accent: '#00c2a8',
  },
  {
    headline: 'GPT-5.3 Codex：AI 参与 AI 开发',
    subline: '关键词：自举开发、终端编程、速度和 token 效率',
    bullets: [
      'OpenAI 披露：早期模型参与训练调试与部署诊断',
      'Terminal-Bench 2.0 分数领先，工程实战导向明显',
      '同任务 token 消耗显著下降，交互迭代更顺滑',
    ],
    accent: '#4d96ff',
  },
  {
    headline: '跑分不应横比：先看基准版本',
    subline: '同名测试，可能不是同一难度与同一题集',
    bullets: [
      'OSWorld 与 OSWorld-Verified 难度与清洗方式不同',
      'SWE-bench Verified 与 Pro Public 覆盖范围不同',
      '结论：先对齐评测口径，再讨论谁更强',
    ],
    accent: '#ffd93d',
  },
  {
    headline: '作者实战工作流（文章观点）',
    subline: '先草拟再精修，双模型协同提效',
    bullets: [
      'Claude：大上下文整理需求、搭骨架、生成草稿',
      'Codex：攻克难 bug、做精细改造与工程落地',
      '核心不是二选一，而是任务分解与编排能力',
    ],
    accent: '#6bcB77',
  },
  {
    headline: '结论：2026 的主线是 Agent 与开发自动化',
    subline: '未来已来，关键在于你的上手速度',
    bullets: [
      '模型差距在缩小，但产品形态差异在扩大',
      '软件行业进入“人类 + Agent 团队”新范式',
      '现在开始系统化实践，胜过围观参数表',
    ],
    accent: '#f06595',
  },
];

const sceneLength = 600;

const bgStyle: React.CSSProperties = {
  background:
    'radial-gradient(circle at 15% 20%, rgba(255,255,255,0.08), transparent 35%), radial-gradient(circle at 85% 80%, rgba(255,255,255,0.07), transparent 30%), linear-gradient(135deg, #0b1021 0%, #121a33 45%, #1a2342 100%)',
};

const textBase: React.CSSProperties = {
  fontFamily:
    'SF Pro Display, PingFang SC, Hiragino Sans GB, Microsoft YaHei, Noto Sans SC, sans-serif',
  color: '#f7f9ff',
};

const SceneCard: React.FC<{scene: Scene; localFrame: number}> = ({scene, localFrame}) => {
  const {fps} = useVideoConfig();

  const cardY = spring({
    frame: localFrame,
    fps,
    config: {
      damping: 18,
      stiffness: 130,
      mass: 0.9,
    },
  });

  const cardOpacity = interpolate(localFrame, [0, 18, 540, 590], [0, 1, 1, 0], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
    easing: Easing.bezier(0.4, 0, 0.2, 1),
  });

  const headlineChars = Math.floor(interpolate(localFrame, [8, 90], [0, scene.headline.length], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  }));

  const sublineOpacity = interpolate(localFrame, [70, 130], [0, 1], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });

  return (
    <AbsoluteFill
      style={{
        justifyContent: 'center',
        alignItems: 'center',
        opacity: cardOpacity,
        transform: `translateY(${(1 - cardY) * 40}px)`,
        ...textBase,
      }}
    >
      <div
        style={{
          width: 1500,
          borderRadius: 28,
          padding: '62px 78px',
          background: 'rgba(12, 16, 36, 0.72)',
          boxShadow: `0 20px 60px ${scene.accent}40`,
          border: `1px solid ${scene.accent}77`,
          backdropFilter: 'blur(2px)',
        }}
      >
        <div
          style={{
            width: 120,
            height: 8,
            borderRadius: 999,
            backgroundColor: scene.accent,
            marginBottom: 28,
          }}
        />
        <div
          style={{
            fontSize: 66,
            lineHeight: 1.16,
            fontWeight: 760,
            letterSpacing: -1.2,
            minHeight: 152,
          }}
        >
          {scene.headline.slice(0, headlineChars)}
        </div>
        <div
          style={{
            marginTop: 18,
            fontSize: 36,
            lineHeight: 1.35,
            opacity: sublineOpacity,
            color: '#d7def7',
          }}
        >
          {scene.subline}
        </div>

        <div style={{marginTop: 44, display: 'grid', gap: 16}}>
          {scene.bullets.map((bullet, idx) => {
            const delay = 130 + idx * 28;
            const rowOpacity = interpolate(localFrame, [delay, delay + 24], [0, 1], {
              extrapolateLeft: 'clamp',
              extrapolateRight: 'clamp',
            });
            const rowX = interpolate(localFrame, [delay, delay + 24], [24, 0], {
              extrapolateLeft: 'clamp',
              extrapolateRight: 'clamp',
            });
            return (
              <div
                key={bullet}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  opacity: rowOpacity,
                  transform: `translateX(${rowX}px)`,
                }}
              >
                <div
                  style={{
                    width: 12,
                    height: 12,
                    borderRadius: 999,
                    backgroundColor: scene.accent,
                    marginRight: 16,
                    marginTop: 3,
                    flexShrink: 0,
                  }}
                />
                <div style={{fontSize: 33, lineHeight: 1.35, color: '#eef2ff'}}>{bullet}</div>
              </div>
            );
          })}
        </div>
      </div>
    </AbsoluteFill>
  );
};

export const AIDuelVideo: React.FC = () => {
  const frame = useCurrentFrame();
  const sceneIndex = Math.min(Math.floor(frame / sceneLength), scenes.length - 1);
  const localFrame = frame - sceneIndex * sceneLength;

  const pulse = interpolate(frame % 120, [0, 60, 119], [0.5, 1, 0.5], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });

  return (
    <AbsoluteFill style={bgStyle}>
      <AbsoluteFill
        style={{
          opacity: 0.25,
          transform: `scale(${0.95 + pulse * 0.08})`,
          background:
            'conic-gradient(from 180deg at 50% 50%, rgba(255,255,255,0.08), rgba(255,255,255,0.02), rgba(255,255,255,0.08))',
        }}
      />
      <SceneCard scene={scenes[sceneIndex]} localFrame={localFrame} />

      <div
        style={{
          position: 'absolute',
          left: 60,
          bottom: 44,
          ...textBase,
          fontSize: 24,
          color: '#c7d0ef',
          letterSpacing: 0.2,
        }}
      >
        文章总结视频 | Claude Opus 4.6 vs GPT-5.3 Codex | 2026-02-12
      </div>

      <div
        style={{
          position: 'absolute',
          right: 60,
          bottom: 42,
          ...textBase,
          fontSize: 24,
          color: '#dfe6ff',
          minWidth: 260,
          textAlign: 'right',
          fontVariantNumeric: 'tabular-nums',
        }}
      >
        {String(Math.floor(frame / 30 / 60)).padStart(2, '0')}:
        {String(Math.floor((frame / 30) % 60)).padStart(2, '0')}
      </div>
    </AbsoluteFill>
  );
};
