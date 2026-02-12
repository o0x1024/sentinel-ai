import React from 'react';
import {useCurrentFrame, useVideoConfig, interpolate} from 'remotion';

interface TypewriterTextProps {
  text: string;
  startFrame?: number;
  durationInFrames?: number;
  fontSize?: number;
  color?: string;
  fontFamily?: string;
}

export const TypewriterText: React.FC<TypewriterTextProps> = ({
  text,
  startFrame = 0,
  durationInFrames = 60,
  fontSize = 48,
  color = '#ffffff',
  fontFamily = 'Arial, sans-serif'
}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  
  // 计算打字机进度
  const progress = interpolate(
    frame,
    [startFrame, startFrame + durationInFrames],
    [0, 1],
    {extrapolateRight: 'clamp'}
  );
  
  // 计算显示的字符数
  const charsToShow = Math.floor(text.length * progress);
  const displayedText = text.substring(0, charsToShow);
  
  // 光标闪烁效果
  const cursorOpacity = Math.sin(frame / 10) > 0 ? 1 : 0;
  
  return (
    <div style={{
      fontFamily,
      fontSize: `${fontSize}px`,
      color,
      fontWeight: 'bold',
      textAlign: 'center',
      lineHeight: 1.2
    }}>
      {displayedText}
      <span style={{
        opacity: cursorOpacity,
        marginLeft: '2px',
        borderRight: `2px solid ${color}`
      }}>|</span>
    </div>
  );
};
