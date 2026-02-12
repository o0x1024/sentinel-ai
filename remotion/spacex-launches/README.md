# SpaceX Launches Animation (2015-2025)

An abstract, minimalist animation visualizing every SpaceX rocket launch from 2015 to 2025 in chronological order. Shows launch parabolas with trajectory fading effects.

## Three Versions Available

### Version 1: Ultra-Minimalist Monochrome
- Pure black background
- White trajectories and rockets
- No glow effects
- Clean, stark aesthetic

### Version 2: Space Blue Gradient
- Deep blue gradient background
- Blue gradient trajectories
- Glow effects on rockets and trails
- Subtle space-themed aesthetic

### Version 3: Warm Energy Theme
- Dark red/black radial gradient
- Orange-to-red gradient trajectories
- Strong glow effects
- Energetic, fiery aesthetic

## Features

- **Chronological Ordering**: Launches displayed in exact historical order
- **Parabolic Trajectories**: Realistic rocket flight paths
- **Fading Trails**: Trajectories fade out over time
- **Launch Details**: Name, date, flight number, success status
- **Progress Tracking**: Shows launch count and completion percentage
- **Year Indicators**: Displays year for each launch
- **Enhanced Animations**: Smooth easing and realistic physics

## Technical Details

- Built with Remotion (React for videos)
- 1920x1080 resolution at 30fps
- Each launch animation: 120 frames (4 seconds)
- Total duration for 10 launches: ~750 frames (25 seconds)
- Real SpaceX API data integrated

## How to Run

1. Install dependencies:
   ```bash
   npm install
   ```

2. Preview animations:
   ```bash
   npm run preview
   ```

3. Render specific version:
   ```bash
   # Render Version 1
   npx remotion render src/index.ts SpaceXLaunches-Version1-Enhanced ../../dist/spacex-version1.mp4

   # Render Version 2  
   npx remotion render src/index.ts SpaceXLaunches-Version2-Enhanced ../../dist/spacex-version2.mp4

   # Render Version 3
   npx remotion render src/index.ts SpaceXLaunches-Version3-Enhanced ../../dist/spacex-version3.mp4
   ```

## Data Source

Uses real SpaceX launch data from the SpaceX API (https://api.spacexdata.com/v4/launches). Currently includes the first 10 launches from 2015-2016 as a demonstration. Can be expanded to include all 187 launches from 2015-2025.

## Customization

The animation can be customized by:
- Changing the `version` prop (1, 2, or 3)
- Modifying color schemes in `SpaceXLaunchesEnhanced.tsx`
- Adjusting animation timing and easing
- Adding more launch data from the SpaceX API
