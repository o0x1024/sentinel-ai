import {Composition} from 'remotion';
import {AISpringFestivalVideo} from './AISpringFestivalVideo';

export const RemotionRoot = () => {
  return (
    <>
      <Composition
        id="AISpringFestival"
        component={AISpringFestivalVideo}
        durationInFrames={3600} // 2分钟 * 30fps = 3600帧
        fps={30}
        width={1080}
        height={1080}
        defaultProps={{
          title: "AI春晚：Claude Opus 4.6 vs GPT-5.3 Codex",
          subtitle: "历史性对决，AI行业大事件"
        }}
      />
    </>
  );
};
