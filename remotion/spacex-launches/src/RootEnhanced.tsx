import React from 'react';
import {Composition} from 'remotion';
import {SpaceXLaunchesEnhanced} from './SpaceXLaunchesEnhanced';

export const RemotionRoot: React.FC = () => {
  return (
    <>
      <Composition
        id="SpaceXLaunches-Version1-Enhanced"
        component={SpaceXLaunchesEnhanced}
        durationInFrames={750}
        fps={30}
        width={1920}
        height={1080}
        defaultProps={{
          version: 1,
        }}
      />
      <Composition
        id="SpaceXLaunches-Version2-Enhanced"
        component={SpaceXLaunchesEnhanced}
        durationInFrames={750}
        fps={30}
        width={1920}
        height={1080}
        defaultProps={{
          version: 2,
        }}
      />
      <Composition
        id="SpaceXLaunches-Version3-Enhanced"
        component={SpaceXLaunchesEnhanced}
        durationInFrames={750}
        fps={30}
        width={1920}
        height={1080}
        defaultProps={{
          version: 3,
        }}
      />
    </>
  );
};
