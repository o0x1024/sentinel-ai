import {Composition} from 'remotion';
import {AIDuelVideo} from './AIDuelVideo';

export const RemotionRoot: React.FC = () => {
  return (
    <Composition
      id="AIDuel2026"
      component={AIDuelVideo}
      durationInFrames={3600}
      fps={30}
      width={1920}
      height={1080}
    />
  );
};
