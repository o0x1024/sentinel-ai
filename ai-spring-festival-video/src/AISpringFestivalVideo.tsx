import React from 'react';
import {useCurrentFrame, useVideoConfig, interpolate, spring} from 'remotion';
import {TypewriterText} from './components/TypewriterText';
import {FadeIn} from './components/FadeIn';
import {DataChart} from './components/DataChart';

interface AISpringFestivalVideoProps {
  title: string;
  subtitle: string;
}

export const AISpringFestivalVideo: React.FC<AISpringFestivalVideoProps> = ({
  title,
  subtitle
}) => {
  const frame = useCurrentFrame();
  const {fps, width, height} = useVideoConfig();
  
  // 场景时间定义（帧数）
  const SCENE_DURATIONS = {
    TITLE: 450,      // 15秒
    OVERVIEW: 450,   // 15秒
    CLAUDE: 900,     // 30秒
    GPT: 900,        // 30秒
    COMPARISON: 450, // 15秒
    CONCLUSION: 450  // 15秒
  };
  
  const sceneStarts = {
    TITLE: 0,
    OVERVIEW: SCENE_DURATIONS.TITLE,
    CLAUDE: SCENE_DURATIONS.TITLE + SCENE_DURATIONS.OVERVIEW,
    GPT: SCENE_DURATIONS.TITLE + SCENE_DURATIONS.OVERVIEW + SCENE_DURATIONS.CLAUDE,
    COMPARISON: SCENE_DURATIONS.TITLE + SCENE_DURATIONS.OVERVIEW + SCENE_DURATIONS.CLAUDE + SCENE_DURATIONS.GPT,
    CONCLUSION: SCENE_DURATIONS.TITLE + SCENE_DURATIONS.OVERVIEW + SCENE_DURATIONS.CLAUDE + SCENE_DURATIONS.GPT + SCENE_DURATIONS.COMPARISON
  };
  
  // 粒子背景动画
  const particleOpacity = interpolate(
    frame % 180,
    [0, 90, 180],
    [0.3, 0.7, 0.3],
    {extrapolateRight: 'clamp'}
  );
  
  // 确定当前场景
  let currentScene = 'TITLE';
  if (frame >= sceneStarts.OVERVIEW && frame < sceneStarts.CLAUDE) currentScene = 'OVERVIEW';
  else if (frame >= sceneStarts.CLAUDE && frame < sceneStarts.GPT) currentScene = 'CLAUDE';
  else if (frame >= sceneStarts.GPT && frame < sceneStarts.COMPARISON) currentScene = 'GPT';
  else if (frame >= sceneStarts.COMPARISON && frame < sceneStarts.CONCLUSION) currentScene = 'COMPARISON';
  else if (frame >= sceneStarts.CONCLUSION) currentScene = 'CONCLUSION';
  
  // 渲染粒子背景
  const renderParticles = () => {
    const particles = [];
    for (let i = 0; i < 50; i++) {
      const x = (Math.sin(frame / 100 + i) * 0.5 + 0.5) * width;
      const y = (Math.cos(frame / 80 + i) * 0.5 + 0.5) * height;
      const size = 2 + Math.sin(frame / 60 + i) * 2;
      
      particles.push(
        <div
          key={i}
          style={{
            position: 'absolute',
            left: x,
            top: y,
            width: size,
            height: size,
            backgroundColor: `rgba(100, 200, 255, ${particleOpacity})`,
            borderRadius: '50%',
            filter: 'blur(1px)'
          }}
        />
      );
    }
    return particles;
  };
  
  // 场景渲染
  const renderScene = () => {
    const sceneFrame = frame - sceneStarts[currentScene as keyof typeof sceneStarts];
    
    switch(currentScene) {
      case 'TITLE':
        return (
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            gap: '40px'
          }}>
            <FadeIn startFrame={0} durationInFrames={30}>
              <TypewriterText
                text={title}
                startFrame={0}
                durationInFrames={90}
                fontSize={64}
                color="#4FC3F7"
              />
            </FadeIn>
            
            <FadeIn startFrame={60} durationInFrames={30} delay={30}>
              <TypewriterText
                text={subtitle}
                startFrame={60}
                durationInFrames={60}
                fontSize={36}
                color="#FFFFFF"
              />
            </FadeIn>
            
            <FadeIn startFrame={150} durationInFrames={60}>
              <div style={{
                fontSize: '24px',
                color: '#90CAF9',
                textAlign: 'center',
                marginTop: '40px'
              }}>
                2026年2月5日 · AI行业历史性时刻
              </div>
            </FadeIn>
          </div>
        );
        
      case 'OVERVIEW':
        return (
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            gap: '30px'
          }}>
            <FadeIn startFrame={0} durationInFrames={30}>
              <TypewriterText
                text="凌晨2点：Claude Opus 4.6发布"
                startFrame={0}
                durationInFrames={60}
                fontSize={48}
                color="#FF9800"
              />
            </FadeIn>
            
            <FadeIn startFrame={60} durationInFrames={30} delay={30}>
              <TypewriterText
                text="20分钟后：GPT-5.3 Codex发布"
                startFrame={60}
                durationInFrames={60}
                fontSize={48}
                color="#2196F3"
              />
            </FadeIn>
            
            <FadeIn startFrame={150} durationInFrames={60}>
              <div style={{
                fontSize: '32px',
                color: '#FFFFFF',
                textAlign: 'center',
                marginTop: '40px',
                fontWeight: 'bold'
              }}>
                "中门对狙！真正的AI春晚"
              </div>
            </FadeIn>
          </div>
        );
        
      case 'CLAUDE':
        return (
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            gap: '40px'
          }}>
            <FadeIn startFrame={0} durationInFrames={30}>
              <TypewriterText
                text="Claude Opus 4.6 技术突破"
                startFrame={0}
                durationInFrames={60}
                fontSize={48}
                color="#FF9800"
              />
            </FadeIn>
            
            <div style={{
              display: 'flex',
              gap: '40px',
              justifyContent: 'center',
              alignItems: 'flex-end'
            }}>
              <DataChart
                label="上下文窗口"
                value={1000}
                maxValue={1000}
                color="#FF9800"
                startFrame={60}
                durationInFrames={120}
              />
              
              <DataChart
                label="ARC-AGI-2测试"
                value={68.8}
                maxValue={100}
                color="#FF9800"
                startFrame={90}
                durationInFrames={120}
              />
              
              <DataChart
                label="Agent Teams"
                value={85}
                maxValue={100}
                color="#FF9800"
                startFrame={120}
                durationInFrames={120}
              />
            </div>
            
            <FadeIn startFrame={240} durationInFrames={60}>
              <div style={{
                fontSize: '24px',
                color: '#FFCC80',
                textAlign: 'center',
                maxWidth: '800px'
              }}>
                1M上下文窗口 · 流体智力突破 · 团队协作模式
              </div>
            </FadeIn>
          </div>
        );
        
      case 'GPT':
        return (
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            gap: '40px'
          }}>
            <FadeIn startFrame={0} durationInFrames={30}>
              <TypewriterText
                text="GPT-5.3 Codex 创新革命"
                startFrame={0}
                durationInFrames={60}
                fontSize={48}
                color="#2196F3"
              />
            </FadeIn>
            
            <div style={{
              display: 'flex',
              gap: '40px',
              justifyContent: 'center',
              alignItems: 'flex-end'
            }}>
              <DataChart
                label="AI参与自身开发"
                value={95}
                maxValue={100}
                color="#2196F3"
                startFrame={60}
                durationInFrames={120}
              />
              
              <DataChart
                label="Token效率提升"
                value={50}
                maxValue={100}
                color="#2196F3"
                startFrame={90}
                durationInFrames={120}
              />
              
              <DataChart
                label="运行速度提升"
                value={25}
                maxValue={100}
                color="#2196F3"
                startFrame={120}
                durationInFrames={120}
              />
            </div>
            
            <FadeIn startFrame={240} durationInFrames={60}>
              <div style={{
                fontSize: '24px',
                color: '#90CAF9',
                textAlign: 'center',
                maxWidth: '800px'
              }}>
                "第一个在创造自己的过程中发挥重要作用的模型"
              </div>
            </FadeIn>
          </div>
        );
        
      case 'COMPARISON':
        return (
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            gap: '40px'
          }}>
            <FadeIn startFrame={0} durationInFrames={30}>
              <TypewriterText
                text="技术对比分析"
                startFrame={0}
                durationInFrames={60}
                fontSize={48}
                color="#9C27B0"
              />
            </FadeIn>
            
            <div style={{
              display: 'flex',
              gap: '60px',
              justifyContent: 'center',
              alignItems: 'center'
            }}>
              <div style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: '20px'
              }}>
                <div style={{
                  fontSize: '32px',
                  color: '#FF9800',
                  fontWeight: 'bold'
                }}>
                  Claude
                </div>
                <div style={{
                  fontSize: '24px',
                  color: '#FFCC80'
                }}>
                  SWE-bench Verified
                </div>
                <div style={{
                  fontSize: '48px',
                  color: '#FF9800',
                  fontWeight: 'bold'
                }}>
                  80.8%
                </div>
              </div>
              
              <div style={{
                fontSize: '36px',
                color: '#FFFFFF',
                margin: '0 20px'
              }}>
                VS
              </div>
              
              <div style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: '20px'
              }}>
                <div style={{
                  fontSize: '32px',
                  color: '#2196F3',
                  fontWeight: 'bold'
                }}>
                  GPT
                </div>
                <div style={{
                  fontSize: '24px',
                  color: '#90CAF9'
                }}>
                  SWE-bench Pro Public
                </div>
                <div style={{
                  fontSize: '48px',
                  color: '#2196F3',
                  fontWeight: 'bold'
                }}>
                  56.8%
                </div>
              </div>
            </div>
            
            <FadeIn startFrame={180} durationInFrames={60}>
              <div style={{
                fontSize: '24px',
                color: '#CE93D8',
                textAlign: 'center',
                maxWidth: '800px'
              }}>
                GPT测试环境更严格 · 含金量更高 · 不可盲目对比
              </div>
            </FadeIn>
          </div>
        );
        
      case 'CONCLUSION':
        return (
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            height: '100%',
            gap: '40px'
          }}>
            <FadeIn startFrame={0} durationInFrames={30}>
              <TypewriterText
                text="行业影响与未来展望"
                startFrame={0}
                durationInFrames={60}
                fontSize={48}
                color="#4CAF50"
              />
            </FadeIn>
            
            <FadeIn startFrame={60} durationInFrames={30}>
              <div style={{
                fontSize: '32px',
                color: '#FFFFFF',
                textAlign: 'center',
                maxWidth: '800px',
                lineHeight: 1.5
              }}>
                软件行业正在经历从诞生以来最大的一次范式转变
              </div>
            </FadeIn>
            
            <FadeIn startFrame={120} durationInFrames={30}>
              <div style={{
                fontSize: '28px',
                color: '#A5D6A7',
                textAlign: 'center',
                maxWidth: '800px',
                lineHeight: 1.5
              }}>
                Agent化趋势 · 团队协作 · AI参与开发
              </div>
            </FadeIn>
            
            <FadeIn startFrame={180} durationInFrames={60}>
              <div style={{
                fontSize: '36px',
                color: '#4FC3F7',
                textAlign: 'center',
                maxWidth: '800px',
                fontWeight: 'bold',
                marginTop: '40px'
              }}>
                "未来已经来了，只是还没均匀分布"
              </div>
            </FadeIn>
            
            <FadeIn startFrame={240} durationInFrames={60}>
              <div style={{
                fontSize: '24px',
                color: '#B0BEC5',
                textAlign: 'center',
                marginTop: '20px'
              }}>
                错过这一波，可能就真的错过了
              </div>
            </FadeIn>
          </div>
        );
        
      default:
        return null;
    }
  };
  
  return (
    <div style={{
      flex: 1,
      backgroundColor: '#0A1929',
      color: 'white',
      fontFamily: 'Arial, sans-serif',
      position: 'relative',
      overflow: 'hidden'
    }}>
      {/* 粒子背景 */}
      {renderParticles()}
      
      {/* 主内容 */}
      <div style={{
        position: 'relative',
        zIndex: 1,
        height: '100%',
        width: '100%'
      }}>
        {renderScene()}
      </div>
      
      {/* 进度指示器 */}
      <div style={{
        position: 'absolute',
        bottom: '20px',
        left: '50%',
        transform: 'translateX(-50%)',
        display: 'flex',
        gap: '10px'
      }}>
        {Object.keys(SCENE_DURATIONS).map((scene, index) => {
          const isActive = currentScene === scene;
          const sceneProgress = Math.min(
            Math.max((frame - sceneStarts[scene as keyof typeof sceneStarts]) / SCENE_DURATIONS[scene as keyof typeof SCENE_DURATIONS], 0),
            1
          );
          
          return (
            <div
              key={scene}
              style={{
                width: '40px',
                height: '4px',
                backgroundColor: isActive ? '#4FC3F7' : 'rgba(255, 255, 255, 0.3)',
                borderRadius: '2px',
                overflow: 'hidden'
              }}
            >
              {isActive && (
                <div
                  style={{
                    width: `${sceneProgress * 100}%`,
                    height: '100%',
                    backgroundColor: '#2196F3',
                    transition: 'width 0.1s ease-out'
                  }}
                />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
};
