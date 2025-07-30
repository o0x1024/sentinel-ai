import { ref, reactive, onMounted, onUnmounted, computed, Ref } from 'vue';

export default function useDraggableResizable(
  targetRef: Ref<HTMLElement | null>,
  options = { initialWidth: 800, initialHeight: 600, minWidth: 400, minHeight: 300 }
) {
  const windowWidth = ref(options.initialWidth);
  const windowHeight = ref(options.initialHeight);
  const windowX = ref((window.innerWidth - options.initialWidth) / 2);
  const windowY = ref((window.innerHeight - options.initialHeight) / 2);
  const isLeftSide = computed(() => windowX.value < window.innerWidth / 2);

  const state = reactive({
    isDragging: false,
    isResizing: false,
    resizeDirection: '',
    startX: 0,
    startY: 0,
    startWidth: 0,
    startHeight: 0,
    startLeft: 0,
    startTop: 0
  });

  const windowStyle = computed(() => ({
    width: `${windowWidth.value}px`,
    height: `${windowHeight.value}px`,
    left: `${windowX.value}px`,
    top: `${windowY.value}px`,
  }));
  
  const startDrag = (e: MouseEvent) => {
    if (e.button !== 0) return;
    state.isDragging = true;
    state.startX = e.clientX;
    state.startY = e.clientY;
    state.startLeft = windowX.value;
    state.startTop = windowY.value;

    document.addEventListener('mousemove', onDrag);
    document.addEventListener('mouseup', stopDrag);
  };

  const onDrag = (e: MouseEvent) => {
    if (!state.isDragging) return;
    const deltaX = e.clientX - state.startX;
    const deltaY = e.clientY - state.startY;
    
    const newX = state.startLeft + deltaX;
    const newY = state.startTop + deltaY;

    // Prevent dragging outside of the window
    const maxX = window.innerWidth - windowWidth.value;
    const maxY = window.innerHeight - windowHeight.value;

    windowX.value = Math.max(0, Math.min(newX, maxX));
    windowY.value = Math.max(0, Math.min(newY, maxY));
  };

  const stopDrag = () => {
    if (state.isDragging) {
      savePosition();
    }
    state.isDragging = false;
    state.isResizing = false;
    document.removeEventListener('mousemove', onDrag);
    document.removeEventListener('mouseup', stopDrag);
    document.removeEventListener('mousemove', onResize);
  };

  const startResize = (direction: string, e: MouseEvent) => {
    if (e.button !== 0) return;
    state.isResizing = true;
    state.resizeDirection = direction;
    state.startX = e.clientX;
    state.startY = e.clientY;
    state.startWidth = windowWidth.value;
    state.startHeight = windowHeight.value;
    state.startLeft = windowX.value;
    state.startTop = windowY.value;

    document.addEventListener('mousemove', onResize);
    document.addEventListener('mouseup', stopDrag);
  };
  
  const onResize = (e: MouseEvent) => {
    if (!state.isResizing) return;
    const deltaX = e.clientX - state.startX;
    const deltaY = e.clientY - state.startY;

    if (state.resizeDirection.includes('e')) {
      windowWidth.value = Math.max(options.minWidth, state.startWidth + deltaX);
    }
    if (state.resizeDirection.includes('w')) {
      const newWidth = Math.max(options.minWidth, state.startWidth - deltaX);
      if(newWidth > options.minWidth) {
        windowX.value = state.startLeft + deltaX;
        windowWidth.value = newWidth;
      }
    }
    if (state.resizeDirection.includes('s')) {
      windowHeight.value = Math.max(options.minHeight, state.startHeight + deltaY);
    }
    if (state.resizeDirection.includes('n')) {
        const newHeight = Math.max(options.minHeight, state.startHeight - deltaY);
        if(newHeight > options.minHeight) {
            windowY.value = state.startTop + deltaY;
            windowHeight.value = newHeight;
        }
    }
  };

  const savePosition = () => {
    const position = { x: windowX.value, y: windowY.value, width: windowWidth.value, height: windowHeight.value };
    localStorage.setItem('chat-window-position', JSON.stringify(position));
  };
  
  const ensureInBounds = () => {
    const maxX = window.innerWidth - windowWidth.value;
    const maxY = window.innerHeight - windowHeight.value;

    windowX.value = Math.max(0, Math.min(windowX.value, maxX));
    windowY.value = Math.max(0, Math.min(windowY.value, maxY));
  };
  
  const loadPosition = () => {
    const saved = localStorage.getItem('chat-window-position');
    if (saved) {
      const pos = JSON.parse(saved);
      windowX.value = pos.x;
      windowY.value = pos.y;
      windowWidth.value = pos.width;
      windowHeight.value = pos.height;
    }
  };
  
  onMounted(() => {
      loadPosition();
      ensureInBounds();
      window.addEventListener('resize', ensureInBounds);
      window.addEventListener('beforeunload', savePosition);
  });
  
  onUnmounted(() => {
      savePosition();
      window.removeEventListener('resize', ensureInBounds);
      window.removeEventListener('beforeunload', savePosition);
  });

  return {
    windowStyle,
    isLeftSide,
    startDrag,
    startResize,
  };
} 