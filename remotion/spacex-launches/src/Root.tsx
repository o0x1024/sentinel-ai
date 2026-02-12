import React from 'react';
import {Composition} from 'remotion';
import {SpaceXLaunches} from './SpaceXLaunches';

export const RemotionRoot: React.FC = () => {
  return (
    <>
      <Composition
        id="SpaceXLaunches-Version1"
        component={SpaceXLaunches}
        durationInFrames={300}
        fps={30}
        width={1920}
        height={1080}
        defaultProps={{
          version: 1,
        }}
      />
      <Composition
        id="SpaceXLaunches-Version2"
        component={SpaceXLaunches}
        durationInFrames={300}
        fps={30}
        width={1920}
        height={1080}
        defaultProps={{
          version: 2,
        }}
      />
      <Composition
        id="SpaceXLaunches-Version3"
        component={SpaceXLaunches}
        durationInFrames={300}
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
