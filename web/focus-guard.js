'use strict';
// Install before the pinned loader: its startup focus must not scroll the article.
(() => {
  const gameCanvas = document.getElementById('glcanvas');
  const focusCanvas = gameCanvas.focus.bind(gameCanvas);
  gameCanvas.focus = (options = {}) => focusCanvas({ ...options, preventScroll: true });
})();
