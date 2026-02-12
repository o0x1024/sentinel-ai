import React from 'react';
import {useCurrentFrame, useVideoConfig, interpolate} from 'remotion';

interface FadeInProps {
  children: React.ReactNode;
  startFrame?: number;
  durationInFrames?: number;
  delay?: number;
}

export const FadeIn: React.FC<FadeInProps> = ({
  children,
  startFrame = 0,
  durationInFrames = 30,
  delay = 0
}) => {
  const frame = useCurrentFrame();
  const {fps} = useVideoConfig();
  
  const opacity = interpolate(
    frame,
    [startFrame + delay, startFrame + delay + durationInFrames],
    [0, 1],
    {extrapolateRight: 'clamp'}
  );
  
  return (
    <div style={{opacity}}>
      {children}
    </div>
  );
};
