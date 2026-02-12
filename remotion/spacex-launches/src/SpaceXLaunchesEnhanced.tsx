import React from 'react';
import {AbsoluteFill, useCurrentFrame, interpolate} from 'remotion';
import {sortedLaunches} from './launchData';

interface LaunchData {
  id: string;
  name: string;
  date_utc: string;
  success: boolean;
  flight_number: number;
  details?: string;
}

interface TrajectoryPoint {
  x: number;
  y: number;
  opacity: number;
  width: number;
}

interface LaunchAnimationProps {
  launch: LaunchData;
  index: number;
  totalLaunches: number;
  version: number;
}

const LaunchAnimation: React.FC<LaunchAnimationProps> = ({
  launch,
  index,
  totalLaunches,
  version,
}) => {
  const frame = useCurrentFrame();
  const startFrame = index * 45; // More space between launches
  const duration = 120; // Longer duration for better visualization
  
  if (frame < startFrame || frame > startFrame + duration) {
    return null;
  }
  
  const localFrame = frame - startFrame;
  
  // Enhanced parabolic trajectory with fading
  const trajectoryPoints: TrajectoryPoint[] = [];
  const pointsCount = 30; // More points for smoother curve
  
  for (let i = 0; i < pointsCount; i++) {
    const progress = i / pointsCount;
    const t = localFrame - i * 1.5; // Slower fade for longer trails
    
    if (t < 0) continue;
    
    // Enhanced parabolic trajectory with more realistic curve
    const x = progress;
    const y = -Math.pow(x, 1.5) + 1.8 * x; // Adjusted curve
    
    // Fade out with exponential decay
    const opacity = Math.max(0, Math.exp(-t / 15));
    
    // Line width also fades
    const width = interpolate(opacity, [0, 1], [0.5, 3]);
    
    trajectoryPoints.push({
      x: x * 1600 + 160, // Wider canvas
      y: (1 - y) * 600 + 100, // Adjusted vertical positioning
      opacity,
      width,
    });
  }
  
  // Current rocket position with smooth interpolation
  const rocketProgress = interpolate(localFrame, [0, duration], [0, 1], {
    extrapolateRight: 'clamp',
    easing: (t) => t * t * (3 - 2 * t), // Smooth easing
  });
  
  const rocketX = rocketProgress * 1600 + 160;
  const rocketY = (1 - (-Math.pow(rocketProgress, 1.5) + 1.8 * rocketProgress)) * 600 + 100;
  
  // Rocket rotation based on trajectory tangent
  const rocketRotation = interpolate(rocketProgress, [0, 1], [45, -30]);
  
  // Version-specific styling
  const getVersionStyles = () => {
    switch(version) {
      case 1: // Ultra-minimalist monochrome
        return {
          bg: '#000000',
          trajectory: '#ffffff',
          rocket: '#ffffff',
          text: '#ffffff',
          accent: '#888888',
          glow: false,
          lineStyle: 'solid',
        };
      case 2: // Space blue with subtle gradients
        return {
          bg: 'linear-gradient(135deg, #0a0a2a 0%, #1a1a3a 50%, #0a0a2a 100%)',
          trajectory: 'linear-gradient(90deg, #3b82f6, #60a5fa)',
          rocket: '#60a5fa',
          text: '#dbeafe',
          accent: '#93c5fd',
          glow: true,
          lineStyle: 'gradient',
        };
      case 3: // Warm energy theme
        return {
          bg: 'radial-gradient(circle at 50% 50%, #1a0a0a 0%, #000000 100%)',
          trajectory: 'linear-gradient(90deg, #f97316, #ef4444)',
          rocket: '#ef4444',
          text: '#fca5a5',
          accent: '#f97316',
          glow: true,
          lineStyle: 'gradient',
        };
      default:
        return {
          bg: '#000000',
          trajectory: '#ffffff',
          rocket: '#ffffff',
          text: '#ffffff',
          accent: '#888888',
          glow: false,
          lineStyle: 'solid',
        };
    }
  };
  
  const styles = getVersionStyles();
  
  // Year indicator
  const launchYear = new Date(launch.date_utc).getFullYear();
  const yearOpacity = interpolate(localFrame, [0, 20, duration - 20, duration], [0, 0.3, 0.3, 0]);
  
  return (
    <AbsoluteFill style={{
      background: styles.bg,
    }}>
      {/* Year indicator */}
      <div
        style={{
          position: 'absolute',
          left: 100,
          top: 50,
          color: styles.text,
          fontFamily: 'Arial, sans-serif',
          fontSize: 64,
          fontWeight: 'bold',
          opacity: yearOpacity,
        }}
      >
        {launchYear}
      </div>
      
      {/* Enhanced trajectory line with fading */}
      {trajectoryPoints.map((point, i) => {
        if (i === 0) return null;
        const prevPoint = trajectoryPoints[i - 1];
        const distance = Math.sqrt(Math.pow(point.x - prevPoint.x, 2) + Math.pow(point.y - prevPoint.y, 2));
        
        return (
          <div
            key={i}
            style={{
              position: 'absolute',
              left: prevPoint.x,
              top: prevPoint.y,
              width: distance,
              height: prevPoint.width,
              background: styles.trajectory,
              opacity: prevPoint.opacity,
              transform: `rotate(${Math.atan2(point.y - prevPoint.y, point.x - prevPoint.x)}rad)`,
              transformOrigin: '0 50%',
              filter: styles.glow ? `blur(${interpolate(prevPoint.opacity, [0, 1], [0, 4])}px)` : 'none',
              borderRadius: '1px',
            }}
          />
        );
      })}
      
      {/* Enhanced rocket with trail */}
      <div
        style={{
          position: 'absolute',
          left: rocketX - 8,
          top: rocketY - 8,
          width: 16,
          height: 16,
          backgroundColor: styles.rocket,
          borderRadius: '50%',
          boxShadow: styles.glow ? `0 0 30px ${styles.rocket}, 0 0 60px ${styles.accent}` : 'none',
          transform: `rotate(${rocketRotation}deg)`,
          zIndex: 10,
        }}
      />
      
      {/* Rocket trail effect */}
      <div
        style={{
          position: 'absolute',
          left: rocketX - 4,
          top: rocketY - 4,
          width: 8,
          height: 8,
          backgroundColor: styles.accent,
          borderRadius: '50%',
          opacity: interpolate(localFrame % 10, [0, 5, 10], [0.8, 1, 0.8]),
          boxShadow: `0 0 20px ${styles.accent}`,
          transform: `rotate(${rocketRotation}deg)`,
        }}
      />
      
      {/* Enhanced launch info */}
      <div
        style={{
          position: 'absolute',
          left: 100,
          top: 700,
          color: styles.text,
          fontFamily: 'Arial, sans-serif',
          fontSize: 32,
          opacity: interpolate(localFrame, [0, 15, duration - 15, duration], [0, 1, 1, 0]),
          maxWidth: '800px',
        }}
      >
        <div style={{
          fontWeight: 'bold',
          fontSize: 40,
          marginBottom: 8,
          background: styles.lineStyle === 'gradient' ? styles.trajectory : 'none',
          WebkitBackgroundClip: styles.lineStyle === 'gradient' ? 'text' : 'none',
          WebkitTextFillColor: styles.lineStyle === 'gradient' ? 'transparent' : 'inherit',
        }}>
          {launch.name}
        </div>
        <div style={{fontSize: 20, opacity: 0.8, marginBottom: 4}}>
          {new Date(launch.date_utc).toLocaleDateString('en-US', {
            year: 'numeric',
            month: 'long',
            day: 'numeric',
          })} • Flight #{launch.flight_number}
        </div>
        {launch.details && (
          <div style={{
            fontSize: 16,
            opacity: 0.7,
            fontStyle: 'italic',
            marginTop: 8,
            maxWidth: '600px',
          }}>
            {launch.details}
          </div>
        )}
        <div style={{
          fontSize: 18,
          color: launch.success ? '#10b981' : '#ef4444',
          marginTop: 12,
          fontWeight: 'bold',
        }}>
          {launch.success ? '✓ SUCCESSFUL LAUNCH' : '✗ LAUNCH FAILURE'}
        </div>
      </div>
      
      {/* Enhanced progress indicator */}
      <div
        style={{
          position: 'absolute',
          right: 100,
          top: 700,
          color: styles.text,
          fontFamily: 'Arial, sans-serif',
          fontSize: 20,
          opacity: interpolate(localFrame, [0, 15, duration - 15, duration], [0, 1, 1, 0]),
          textAlign: 'right',
        }}
      >
        <div style={{fontSize: 24, fontWeight: 'bold', marginBottom: 8}}>
          LAUNCH {index + 1} OF {totalLaunches}
        </div>
        <div style={{
          width: '200px',
          height: '4px',
          backgroundColor: 'rgba(255,255,255,0.2)',
          borderRadius: '2px',
          overflow: 'hidden',
          marginBottom: 8,
        }}>
          <div style={{
            width: `${((index + 1) / totalLaunches) * 100}%`,
            height: '100%',
            background: styles.trajectory,
            transition: 'width 0.3s ease',
          }} />
        </div>
        <div style={{fontSize: 16, opacity: 0.7}}>
          {Math.round(((index + 1) / totalLaunches) * 100)}% complete
        </div>
      </div>
    </AbsoluteFill>
  );
};

interface SpaceXLaunchesEnhancedProps {
  version?: number;
}

export const SpaceXLaunchesEnhanced: React.FC<SpaceXLaunchesEnhancedProps> = ({version = 1}) => {
  const frame = useCurrentFrame();
  const totalDuration = sortedLaunches.length * 45 + 120;
  
  if (frame > totalDuration) {
    return (
      <AbsoluteFill style={{
        background: version === 1 ? '#000000' : 
                   version === 2 ? 'linear-gradient(135deg, #0a0a2a 0%, #1a1a3a 100%)' :
                   'radial-gradient(circle at 50% 50%, #1a0a0a 0%, #000000 100%)',
        color: '#ffffff',
        justifyContent: 'center',
        alignItems: 'center',
        fontFamily: 'Arial, sans-serif',
        textAlign: 'center',
      }}>
        <div style={{fontSize: 72, fontWeight: 'bold', marginBottom: 40}}>
          SPACEX LAUNCHES
        </div>
        <div style={{fontSize: 48, marginBottom: 20}}>
          2015 – 2025
        </div>
        <div style={{
          fontSize: 32,
          opacity: 0.8,
          marginBottom: 60,
          maxWidth: '800px',
          lineHeight: 1.4,
        }}>
          {sortedLaunches.length} launches visualized in chronological order
        </div>
        <div style={{
          fontSize: 24,
          opacity: 0.6,
          fontStyle: 'italic',
        }}>
          Each parabola represents a rocket's trajectory
        </div>
        <div style={{
          fontSize: 20,
          opacity: 0.5,
          marginTop: 40,
        }}>
          Fading trails show the passage of time
        </div>
      </AbsoluteFill>
    );
  }
  
  return (
    <AbsoluteFill>
      {sortedLaunches.map((launch, index) => (
        <LaunchAnimation
          key={launch.id}
          launch={launch}
          index={index}
          totalLaunches={sortedLaunches.length}
          version={version}
        />
      ))}
    </AbsoluteFill>
  );
};
