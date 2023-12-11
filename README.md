# Topola

Work-in-progress interactive topological router in Rust.

Join the [official Matrix chatroom](https://matrix.to/#/%23topola:tchncs.de) to talk with the
developers. 

## Contributing

[Official repository on Codeberg](https://codeberg.org/mikolaj/topola)

[Mirror repository on GitHub](https://github.com/mikwielgus/topola)

## Gallery
![Animation. There's a rubber band-like trace following cursor, navigating a very simple maze. The maze and the trace are red, the background is solid black but also very slightly white and dark blue](./assets/interactive_routing.gif)

![Animation showing a trace, behaving like a rubber band, routed around obstacles step by step. Attempted alternative paths and a guiding mesh are shown.](./assets/mesh_visualization.gif)

![Animation. There are two upward barriers, with some space between tem, around which four rubberband traces, one over another, are wrapped. Enter mouse cursor. The cursor begins to stretch the left barrier to the right. As it's stretched, the traces cease to be wrapped around the right barrier, becoming "free". The traces and the barrier are two-dimensional and all solid red. The background is black but also very slightly white and blue.](./assets/unwrapping_bends.gif "Unwrapping bends")

![This animation shows four traces wrapped around a vertical barrier like rubberbands. Computer
cursor appears and starts dragging the barrier's top end left and right, up and down, elastically
stretching the barrier and having the traces continue being wrapped on the barrier regardless of
its position. The traces and the barrier are all solid red. The background is black but also very
slightly white and blue.](./assets/dragging_with_bends.gif "Dragging with bends")

![Animation. There is an upward barrier in the middle and dots on the left and right of it, four each. A trace is drawn from the leftmost dot on the left to the rightmost dot on the right. Then a trace is drawn from the second leftmost dot on the left to the second rightmost dot on the right, displacing the previous trace so that there's space for the new one. Same happens for the remaining dots. The dots, traces and barrier are all solid red. The background is black but also very slightly white and blue.](./assets/shoving_around.gif "Shoving traces under other traces")

![Animation showing three red-colored traces pass around a barrier. Trace bends are not aligned to a grid unlike most PCB layouts these days (this is called "topological routing"). The traces and the barrier are all solid red. The background is black but also very
slightly white and blue.](./assets/stacked_bends.png "Stacking bends")

![Animation showing a trace zigzagging around two barriers. The trace and the barriers are all solid red. The background is black but also very slightly white and blue.](./assets/zigzag.png "Zigzag")
