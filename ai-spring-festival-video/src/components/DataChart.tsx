import React from 'react';
import {useCurrentFrame, useVideoConfig, interpolate} from 'remotion';

interface DataChartProps {
  label: string;
  value: number;
  maxValue: number;
  color: string;
  startFrame?: number;
  durationInFrames?: number;
  width?: number;
  height?: number;
}

export const DataChart: React.FC<DataChartProps> = ({
  label,
  value,
  maxValue,
  color,
  startFrame = 0,
  durationInFrames = 60,
  width = 400,
  height = 200
}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  
  // 计算动画进度
  const progress = interpolate(
    frame,
    [startFrame, startFrame + durationInFrames],
    [0, 1],
    {extrapolateRight: 'clamp'}
  );
  
  const animatedValue = value * progress;
  const barHeight = (animatedValue / maxValue) * (height - 40);
  
  return (
    <div style={{
      width: `${width}px`,
      height: `${height}px`,
      backgroundColor: 'rgba(0, 0, 0, 0.3)',
      borderRadius: '10px',
      padding: '20px',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'flex-end'
    }}>
      <div style={{
        fontSize: '24px',
        color: '#ffffff',
        marginBottom: '10px',
        fontWeight: 'bold'
      }}>
        {label}
      </div>
      
      <div style={{
        width: '80%',
        height: `${height - 60}px`,
        backgroundColor: 'rgba(255, 255, 255, 0.1)',
        borderRadius: '5px',
        position: 'relative',
        overflow: 'hidden'
      }}>
        <div style={{
          position: 'absolute',
          bottom: 0,
          left: 0,
          width: '100%',
          height: `${barHeight}px`,
          backgroundColor: color,
          transition: 'height 0.1s ease-out'
        }} />
      </div>
      
      <div style={{
        fontSize: '32px',
        color: color,
        marginTop: '10px',
        fontWeight: 'bold'
      }}>
        {animatedValue.toFixed(1)}%
      </div>
    </div>
  );
};
