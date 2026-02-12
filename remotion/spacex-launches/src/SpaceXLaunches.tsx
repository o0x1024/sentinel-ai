import React from 'react';
import {AbsoluteFill, Sequence, useCurrentFrame, interpolate, spring} from 'remotion';

interface LaunchData {
  id: string;
  name: string;
  date_utc: string;
  success: boolean;
  flight_number: number;
}

interface TrajectoryPoint {
  x: number;
  y: number;
  opacity: number;
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
  const startFrame = index * 30;
  const duration = 90;
  
  if (frame < startFrame || frame > startFrame + duration) {
    return null;
  }
  
  const localFrame = frame - startFrame;
  
  // Generate parabolic trajectory
  const trajectoryPoints: TrajectoryPoint[] = [];
  const pointsCount = 20;
  
  for (let i = 0; i < pointsCount; i++) {
    const progress = i / pointsCount;
    const t = localFrame - i * 2;
    
    if (t < 0) continue;
    
    // Parabolic trajectory: y = -x^2 + 2x (normalized to 0-1 range)
    const x = progress;
    const y = -Math.pow(x, 2) + 2 * x;
    
    // Fade out older points
    const opacity = interpolate(t, [0, 20], [1, 0], {
      extrapolateRight: 'clamp',
    });
    
    trajectoryPoints.push({
      x: x * 800 + 100,
      y: (1 - y) * 400 + 50,
      opacity,
    });
  }
  
  // Current rocket position
  const rocketProgress = interpolate(localFrame, [0, duration], [0, 1], {
    extrapolateRight: 'clamp',
  });
  const rocketX = rocketProgress * 800 + 100;
  const rocketY = (1 - (-Math.pow(rocketProgress, 2) + 2 * rocketProgress)) * 400 + 50;
  
  // Version-specific styling
  const getVersionStyles = () => {
    switch(version) {
      case 1: // Minimalist white on dark
        return {
          bg: '#0a0a0a',
          trajectory: '#ffffff',
          rocket: '#ffffff',
          text: '#ffffff',
          glow: false,
        };
      case 2: // Blue gradient
        return {
          bg: 'linear-gradient(135deg, #0f172a 0%, #1e293b 100%)',
          trajectory: '#60a5fa',
          rocket: '#3b82f6',
          text: '#dbeafe',
          glow: true,
        };
      case 3: // Orange/red energy
        return {
          bg: '#000000',
          trajectory: '#f97316',
          rocket: '#ef4444',
          text: '#fca5a5',
          glow: true,
        };
      default:
        return {
          bg: '#0a0a0a',
          trajectory: '#ffffff',
          rocket: '#ffffff',
          text: '#ffffff',
          glow: false,
        };
    }
  };
  
  const styles = getVersionStyles();
  
  return (
    <AbsoluteFill style={{
      background: styles.bg,
    }}>
      {/* Trajectory line */}
      {trajectoryPoints.map((point, i) => {
        if (i === 0) return null;
        const prevPoint = trajectoryPoints[i - 1];
        return (
          <div
            key={i}
            style={{
              position: 'absolute',
              left: prevPoint.x,
              top: prevPoint.y,
              width: Math.sqrt(Math.pow(point.x - prevPoint.x, 2) + Math.pow(point.y - prevPoint.y, 2)),
              height: 1,
              backgroundColor: styles.trajectory,
              opacity: prevPoint.opacity,
              transform: `rotate(${Math.atan2(point.y - prevPoint.y, point.x - prevPoint.x)}rad)`,
              transformOrigin: '0 0',
              filter: styles.glow ? `blur(${interpolate(prevPoint.opacity, [0, 1], [0, 2])}px)` : 'none',
            }}
          />
        );
      })}
      
      {/* Rocket */}
      <div
        style={{
          position: 'absolute',
          left: rocketX - 5,
          top: rocketY - 5,
          width: 10,
          height: 10,
          backgroundColor: styles.rocket,
          borderRadius: '50%',
          boxShadow: styles.glow ? `0 0 20px ${styles.rocket}` : 'none',
          transform: `rotate(${interpolate(rocketProgress, [0, 1], [45, -45])}deg)`,
        }}
      />
      
      {/* Launch info */}
      <div
        style={{
          position: 'absolute',
          left: 50,
          top: 500,
          color: styles.text,
          fontFamily: 'Arial, sans-serif',
          fontSize: 24,
          opacity: interpolate(localFrame, [0, 10, duration - 10, duration], [0, 1, 1, 0]),
        }}
      >
        <div style={{fontWeight: 'bold'}}>{launch.name}</div>
        <div style={{fontSize: 16, opacity: 0.8}}>
          {new Date(launch.date_utc).toLocaleDateString()} • Flight #{launch.flight_number}
        </div>
        <div style={{
          fontSize: 14,
          color: launch.success ? '#10b981' : '#ef4444',
          marginTop: 4,
        }}>
          {launch.success ? '✓ Success' : '✗ Failed'}
        </div>
      </div>
      
      {/* Progress indicator */}
      <div
        style={{
          position: 'absolute',
          right: 50,
          top: 500,
          color: styles.text,
          fontFamily: 'Arial, sans-serif',
          fontSize: 16,
          opacity: interpolate(localFrame, [0, 10, duration - 10, duration], [0, 1, 1, 0]),
        }}
      >
        Launch {index + 1} of {totalLaunches}
      </div>
    </AbsoluteFill>
  );
};

// Mock data for demonstration
const mockLaunches: LaunchData[] = [
  {id: '1', name: 'CRS-5', date_utc: '2015-01-10T09:47:00.000Z', success: true, flight_number: 18},
  {id: '2', name: 'DSCOVR', date_utc: '2015-02-11T23:03:00.000Z', success: true, flight_number: 19},
  {id: '3', name: 'ABS-3A / Eutelsat 115W B', date_utc: '2015-03-02T03:50:00.000Z', success: true, flight_number: 20},
  {id: '4', name: 'CRS-6', date_utc: '2015-04-14T20:10:00.000Z', success: true, flight_number: 21},
  {id: '5', name: 'TürkmenÄlem 52°E / MonacoSAT', date_utc: '2015-04-27T23:03:00.000Z', success: true, flight_number: 22},
];

interface SpaceXLaunchesProps {
  version?: number;
}

export const SpaceXLaunches: React.FC<SpaceXLaunchesProps> = ({version = 1}) => {
  const frame = useCurrentFrame();
  const totalDuration = mockLaunches.length * 30 + 90;
  
  if (frame > totalDuration) {
    return (
      <AbsoluteFill style={{
        backgroundColor: '#0a0a0a',
        color: '#ffffff',
        justifyContent: 'center',
        alignItems: 'center',
        fontFamily: 'Arial, sans-serif',
        fontSize: 48,
      }}>
        SpaceX Launches 2015-2025
        <div style={{fontSize: 24, marginTop: 20, opacity: 0.7}}>
          {mockLaunches.length} launches visualized
        </div>
      </AbsoluteFill>
    );
  }
  
  return (
    <AbsoluteFill>
      {mockLaunches.map((launch, index) => (
        <LaunchAnimation
          key={launch.id}
          launch={launch}
          index={index}
          totalLaunches={mockLaunches.length}
          version={version}
        />
      ))}
    </AbsoluteFill>
  );
};
