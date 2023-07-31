# Topola

Topological router in Rust.

## Gallery

![This animation shows four traces wrapped around a vertical barrier like rubberbands. Computer
cursor appears and starts dragging the barrier's top end left and right, up and down, elastically
stretching the barrier and having the traces continue being wrapped on the barrier regardless of
its position. The traces and the barrier are all solid red. The background is black but also very
slightly white and blue.](./assets/dragging_with_bends.gif "Dragging with bends")

![Animation. There is an upward barrier in the middle and dots on the left and right of it, four each. A trace is drawn from the leftmost dot on the left to the rightmost dot on the right. Then a trace is drawn from the second leftmost dot on the left to the second rightmost dot on the right, displacing the previous trace so that there's space for the new one. Same happens for the remaining dots. The dots, traces and barrier are all solid red. The background is black but also very slightly white and blue.](./assets/shoving_around.gif "Shoving traces under other traces")

![Three red-colored traces pass around a barrier. Trace bends are not aligned to a grid unlike most PCB layouts these days (this is called "topological routing"). The traces and the barrier are all solid red. The background is black but also very
slightly white and blue.](./assets/stacked_bends.png "Stacking bends")

![A trace zigzagging around two barriers. The trace and the barriers are all solid red. The background is black but also very slightly white and blue.](./assets/zigzag.png "Zigzag")
