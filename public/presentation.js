  const App = (function () {
    let state = {
      mode: 'prescription',
      elementCount: 0,
      activeId: null,
      elements: {}
    };
    const difangTypes = ['底 方', '精 一', '精 二', '麻 醉'];
    let currentDifangIdx = 0;
    let drag = { isDragging: false, startX: 0, startY: 0, initX: 0, initY: 0 };
    let canvasPan = { isPanning: false, startX: 0, startY: 0, scrollLeft: 0, scrollTop: 0 };
    const DOM = {
      canvas: document.getElementById('canvasWrapper'),
      stage: document.getElementById('paperStage'),
      paper: document.getElementById('paperArea'),
      container: document.getElementById('dynamicElementsContainer'),
      watermark: document.getElementById('screenWatermark')
    };
    const PAPER_CONFIG = {
      prescription: {
        page: '@page { size: A5 landscape; margin: 0; }',
        widthMm: 210,
        heightMm: 148,
        toggleText: '🔀 切换为：A4 诊断证明'
      },
      diagnosis: {
        page: '@page { size: A4 portrait; margin: 0; }',
        widthMm: 210,
        heightMm: 297,
        toggleText: '🔀 切换为：A5 处方笺'
      }
    };

    function mmToPx(mm) {
      return (Number(mm) * 96) / 25.4;
    }

    function getPaperPixelSize(mode = state.mode) {
      const conf = PAPER_CONFIG[mode === 'diagnosis' ? 'diagnosis' : 'prescription'];
      const fallbackWidth = Math.round(mmToPx(conf.widthMm));
      const fallbackHeight = Math.round(mmToPx(conf.heightMm));
      if (!DOM.paper) {
        return { width: fallbackWidth, height: fallbackHeight };
      }
      const rect = DOM.paper.getBoundingClientRect();
      return {
        width: Math.max(1, Math.round(rect.width || fallbackWidth)),
        height: Math.max(1, Math.round(rect.height || fallbackHeight))
      };
    }

    function setScreenWatermarkVisible(visible) {
      if (!DOM.watermark) return;
      DOM.watermark.classList.toggle('is-hidden', !visible);
    }

    function toggleDifang() {
      currentDifangIdx = (currentDifangIdx + 1) % difangTypes.length;
      const el = document.getElementById('difangBox');
      if (el) el.innerText = difangTypes[currentDifangIdx];
    }

    function generateNoiseElements(w, h) {
      const count = (w * h) / 10;
      let elements = '<g>';
      for (let i = 0; i < count; i++) {
        elements += `<circle cx="${Math.random() * w}" cy="${Math.random() * h}" r="${Math.random() * 1.5}" fill="black" />`;
      }
      return elements + '</g>';
    }

    function escapeHtml(value) {
      return String(value || '').replace(/[&<>'"]/g, ch => ({
        '&': '&amp;',
        '<': '&lt;',
        '>': '&gt;',
        '"': '&quot;',
        "'": '&#39;'
      }[ch]));
    }

    function clampNumber(value, min, max, fallback) {
      const n = Number(value);
      if (!Number.isFinite(n)) return fallback;
      return Math.min(max, Math.max(min, n));
    }

    function sanitizeId(value, fallback) {
      const s = String(value || '').replace(/[^a-zA-Z0-9_-]/g, '');
      return s || fallback;
    }

    function sanitizeColor(value, fallback) {
      const s = String(value || '').trim();
      if (/^#[0-9a-fA-F]{6}$/.test(s)) return s;
      return fallback;
    }

    function normalizeSignFont(font) {
      const allowlist = [
        "'Liu Jian Mao Cao', cursive",
        "'Zhi Mang Xing', cursive",
        "'KaiTi','STKaiti','Kaiti SC',serif",
        "'FangSong','STFangsong','Songti SC',serif",
        "'SimSun','Songti SC',serif"
      ];
      return allowlist.includes(font) ? font : allowlist[0];
    }

    function buildCircleSealSVG(id, text1, text2, grunge) {
      const sealFont = "'宋体','SimSun','Songti SC','STSong',serif";
      const safeId = sanitizeId(id, 'seal');
      const safeText1 = escapeHtml(text1);
      const safeText2 = escapeHtml(text2);
      const safeGrunge = clampNumber(grunge, 0, 100, 30);
      return `
      <svg class="seal-svg" viewBox="0 0 170 170" style="overflow:visible;">
        <defs>
          <path id="path_${safeId}" d="M 35,112 A 56,56 0 1,1 135,112" />
          <mask id="mask_${safeId}"><rect width="100%" height="100%" fill="white" /><g opacity="${safeGrunge / 100}">${generateNoiseElements(170, 170)}</g></mask>
        </defs>
        <g mask="url(#mask_${safeId})" fill="currentColor" stroke="currentColor">
          <circle cx="85" cy="85" r="78" fill="none" stroke-width="5" />
          <polygon points="85,60 90.6,77.3 108.8,77.3 94.1,88 99.7,105.3 85,94.6 70.3,105.3 75.9,88 61.2,77.3 79.4,77.3" stroke="none" />
          <text font-family="${sealFont}" font-weight="bold" font-size="24" letter-spacing="1" text-anchor="middle" stroke="none">
            <textPath xlink:href="#path_${safeId}" startOffset="50%">${safeText1}</textPath>
          </text>
          <text font-family="${sealFont}" font-weight="bold" x="85" y="130" font-size="20" letter-spacing="2" text-anchor="middle" stroke="none">${safeText2}</text>
        </g>
      </svg>`;
    }

    function buildRectSealSVG(id, text1, text2, grunge) {
      const sealFont = "'宋体','SimSun','Songti SC','STSong',serif";
      const safeId = sanitizeId(id, 'seal_rect');
      const safeText1 = escapeHtml(text1);
      const safeText2 = escapeHtml(text2);
      const safeGrunge = clampNumber(grunge, 0, 100, 30);
      const texts = safeText2
        ? `<text x="45" y="17" text-anchor="middle" font-size="16" letter-spacing="1" font-family="${sealFont}" font-weight="bold" stroke="none">${safeText1}</text>
           <text x="45" y="33" text-anchor="middle" font-size="12" letter-spacing="0" font-family="${sealFont}" font-weight="bold" stroke="none">${safeText2}</text>`
        : `<text x="45" y="26" text-anchor="middle" font-size="20" letter-spacing="1" font-family="${sealFont}" font-weight="bold" stroke="none">${safeText1}</text>`;
      return `
      <svg class="seal-svg" viewBox="0 0 90 40" style="overflow:visible;">
        <defs><mask id="mask_${safeId}"><rect width="100%" height="100%" fill="white" /><g opacity="${safeGrunge / 100}">${generateNoiseElements(90, 40)}</g></mask></defs>
        <g mask="url(#mask_${safeId})" fill="currentColor" stroke="currentColor">
          <rect x="2" y="2" width="86" height="36" fill="none" stroke-width="2.5" />
          ${texts}
        </g>
      </svg>`;
    }

    function buildSignatureHTML(data) {
      const font = normalizeSignFont(data.font);
      const size = clampNumber(data.size, 20, 120, 50);
      const spacing = clampNumber(data.spacing, -30, 10, -8);
      const skew = clampNumber(data.skew, -50, 20, -25);
      const text = escapeHtml(data.text1);
      return `<div class="doctor-signature" style="font-family:${font};font-size:${size}px;letter-spacing:${spacing}px;transform:skewX(${skew}deg);filter:${font.includes('Liu Jian Mao Cao') && skew < -10 ? 'blur(0.4px)' : 'none'};">${text}</div>`;
    }

    function addDraggableElement(type, data) {
      state.elementCount++;
      const id = sanitizeId(data.id || `el_${state.elementCount}`, `el_${state.elementCount}`);
      const wrapper = document.createElement('div');
      wrapper.className = 'draggable-item';
      if (type === 'sign') wrapper.classList.add('draggable-sign-container');
      wrapper.id = id;

      const paperSize = getPaperPixelSize(state.mode);
      const paperWidth = paperSize.width;
      const paperHeight = paperSize.height;
      const posX = data.x !== undefined ? clampNumber(data.x, -2000, 4000, 0) : (paperWidth / 2 - 50 + (Math.random() * 100 - 50));
      const posY = data.y !== undefined ? clampNumber(data.y, -2000, 4000, 0) : (paperHeight / 2 - 50 + (Math.random() * 100 - 50));
      const rotate = clampNumber(data.rotate, -180, 180, 0);
      const color = sanitizeColor(data.color, type === 'sign' ? '#1a2233' : '#d90000');
      const grunge = clampNumber(data.grunge, 0, 100, 30);

      wrapper.setAttribute('data-type', type);
      wrapper.setAttribute('data-color', color);
      wrapper.setAttribute('data-grunge', String(grunge));
      state.elements[id] = { x: posX, y: posY, r: rotate };
      wrapper.style.transform = `translate(${posX}px, ${posY}px) rotate(${rotate}deg)`;
      wrapper.style.color = color;

      if (type === 'circle') {
        const text1 = String(data.text1 || 'XX市中心医院');
        const text2 = String(data.text2 || '处方专用章');
        wrapper.setAttribute('data-text1', text1);
        wrapper.setAttribute('data-text2', text2);
        wrapper.innerHTML = buildCircleSealSVG(id, text1, text2, grunge);
        wrapper.style.width = '170px';
        wrapper.style.height = '170px';
      } else if (type === 'rect') {
        const text1 = String(data.text1 || '已发药');
        const text2 = String(data.text2 || '');
        wrapper.setAttribute('data-text1', text1);
        wrapper.setAttribute('data-text2', text2);
        wrapper.innerHTML = buildRectSealSVG(id, text1, text2, grunge);
        wrapper.style.width = '90px';
        wrapper.style.height = '40px';
      } else if (type === 'sign') {
        const text1 = String(data.text1 || 'XXX');
        const font = normalizeSignFont(data.font || "'Liu Jian Mao Cao', cursive");
        const size = clampNumber(data.size, 20, 120, 50);
        const spacing = clampNumber(data.spacing, -30, 10, -8);
        const skew = clampNumber(data.skew, -50, 20, -25);
        wrapper.setAttribute('data-text1', text1);
        wrapper.setAttribute('data-font', font);
        wrapper.setAttribute('data-size', String(size));
        wrapper.setAttribute('data-spacing', String(spacing));
        wrapper.setAttribute('data-skew', String(skew));
        wrapper.style.color = color;
        wrapper.innerHTML = buildSignatureHTML({ text1, font, size, spacing, skew });
      }

      DOM.container.appendChild(wrapper);
      selectElement(wrapper);
      return wrapper;
    }

    function extractElementItems() {
      if (!DOM.container) return [];
      const items = [];
      DOM.container.querySelectorAll('.draggable-item').forEach(el => {
        const id = sanitizeId(el.id, `el_${items.length + 1}`);
        const type = el.getAttribute('data-type');
        const pos = state.elements[id] || {};
        if (!['circle', 'rect', 'sign'].includes(type)) return;
        items.push({
          id,
          type,
          x: clampNumber(pos.x, -2000, 4000, 0),
          y: clampNumber(pos.y, -2000, 4000, 0),
          rotate: clampNumber(pos.r, -180, 180, 0),
          color: sanitizeColor(el.getAttribute('data-color'), type === 'sign' ? '#1a2233' : '#d90000'),
          grunge: clampNumber(el.getAttribute('data-grunge'), 0, 100, 30),
          text1: String(el.getAttribute('data-text1') || ''),
          text2: String(el.getAttribute('data-text2') || ''),
          font: normalizeSignFont(el.getAttribute('data-font') || "'Liu Jian Mao Cao', cursive"),
          size: clampNumber(el.getAttribute('data-size'), 20, 120, 50),
          spacing: clampNumber(el.getAttribute('data-spacing'), -30, 10, -8),
          skew: clampNumber(el.getAttribute('data-skew'), -50, 20, -25)
        });
      });
      return items;
    }

    function buildItemsFromLegacyPayload(payload) {
      const html = String(payload && payload.containerHtml ? payload.containerHtml : '');
      if (!html) return [];
      const template = document.createElement('template');
      template.innerHTML = html;
      const elementsMap = payload && payload.elements && typeof payload.elements === 'object' ? payload.elements : {};
      const items = [];
      template.content.querySelectorAll('.draggable-item').forEach((el, idx) => {
        const id = sanitizeId(el.id, `legacy_${idx + 1}`);
        const type = el.getAttribute('data-type');
        if (!['circle', 'rect', 'sign'].includes(type)) return;
        const pos = elementsMap[id] || {};
        items.push({
          id,
          type,
          x: clampNumber(pos.x, -2000, 4000, 0),
          y: clampNumber(pos.y, -2000, 4000, 0),
          rotate: clampNumber(pos.r, -180, 180, 0),
          color: sanitizeColor(el.getAttribute('data-color'), type === 'sign' ? '#1a2233' : '#d90000'),
          grunge: clampNumber(el.getAttribute('data-grunge'), 0, 100, 30),
          text1: String(el.getAttribute('data-text1') || ''),
          text2: String(el.getAttribute('data-text2') || ''),
          font: normalizeSignFont(el.getAttribute('data-font') || "'Liu Jian Mao Cao', cursive"),
          size: clampNumber(el.getAttribute('data-size'), 20, 120, 50),
          spacing: clampNumber(el.getAttribute('data-spacing'), -30, 10, -8),
          skew: clampNumber(el.getAttribute('data-skew'), -50, 20, -25)
        });
      });
      return items;
    }

    function updateElementTransform(elId) {
      const el = document.getElementById(elId);
      const elState = state.elements[elId];
      if (el && elState) el.style.transform = `translate(${elState.x}px, ${elState.y}px) rotate(${elState.r}deg)`;
    }

    function selectElement(el) {
      if (state.activeId) {
        const prev = document.getElementById(state.activeId);
        if (prev) prev.classList.remove('is-selected');
      }
      state.activeId = el ? el.id : null;
      if (el) el.classList.add('is-selected');

      document.getElementById('noSelectionMsg').classList.add('hidden');
      document.getElementById('circleEditArea').classList.add('hidden');
      document.getElementById('rectEditArea').classList.add('hidden');
      document.getElementById('signEditArea').classList.add('hidden');

      if (!el) {
        document.getElementById('noSelectionMsg').classList.remove('hidden');
        return;
      }

      const type = el.getAttribute('data-type');
      const elState = state.elements[el.id] || {};
      if (type === 'circle') {
        document.getElementById('circleEditArea').classList.remove('hidden');
        document.getElementById('sealTopText').value = el.getAttribute('data-text1') || '';
        document.getElementById('sealBottomText').value = el.getAttribute('data-text2') || '';
        document.getElementById('circleRotate').value = elState.r || 0;
        document.getElementById('circleGrunge').value = el.getAttribute('data-grunge') || '30';
      } else if (type === 'rect') {
        document.getElementById('rectEditArea').classList.remove('hidden');
        document.getElementById('activeSealText').value = el.getAttribute('data-text1') || '';
        document.getElementById('activeSealText2').value = el.getAttribute('data-text2') || '';
        document.getElementById('activeSealRotate').value = elState.r || 0;
        document.getElementById('activeSealGrunge').value = el.getAttribute('data-grunge') || '30';
      } else if (type === 'sign') {
        document.getElementById('signEditArea').classList.remove('hidden');
        document.getElementById('signText').value = el.getAttribute('data-text1') || '';
        document.getElementById('signFont').value = el.getAttribute('data-font') || "'Liu Jian Mao Cao', cursive";
        document.getElementById('signRotate').value = elState.r || 0;
        document.getElementById('signSize').value = parseInt(el.getAttribute('data-size')) || 50;
        document.getElementById('signSpace').value = parseInt(el.getAttribute('data-spacing')) || -8;
        document.getElementById('signSkew').value = parseInt(el.getAttribute('data-skew')) || -25;
        document.getElementById('signColor').value = el.getAttribute('data-color') || '#1a2233';
      }
    }

    function updateActiveElement() {
      if (!state.activeId) return;
      const el = document.getElementById(state.activeId);
      if (!el) return;
      const type = el.getAttribute('data-type');

      if (type === 'circle') {
        const t1 = document.getElementById('sealTopText').value;
        const t2 = document.getElementById('sealBottomText').value;
        const rot = document.getElementById('circleRotate').value;
        const grunge = document.getElementById('circleGrunge').value;
        el.setAttribute('data-text1', t1);
        el.setAttribute('data-text2', t2);
        el.setAttribute('data-grunge', grunge);
        state.elements[el.id].r = rot;
        el.innerHTML = buildCircleSealSVG(el.id, t1, t2, grunge);
      } else if (type === 'rect') {
        const t1 = document.getElementById('activeSealText').value;
        const t2 = document.getElementById('activeSealText2').value;
        const rot = document.getElementById('activeSealRotate').value;
        const grunge = document.getElementById('activeSealGrunge').value;
        el.setAttribute('data-text1', t1);
        el.setAttribute('data-text2', t2);
        el.setAttribute('data-grunge', grunge);
        state.elements[el.id].r = rot;
        el.innerHTML = buildRectSealSVG(el.id, t1, t2, grunge);
      } else if (type === 'sign') {
        const text = document.getElementById('signText').value;
        const font = document.getElementById('signFont').value;
        const rotate = document.getElementById('signRotate').value;
        const size = document.getElementById('signSize').value;
        const space = document.getElementById('signSpace').value;
        const skew = document.getElementById('signSkew').value;
        const color = document.getElementById('signColor').value;
        el.setAttribute('data-text1', text);
        el.setAttribute('data-font', font);
        el.setAttribute('data-size', size);
        el.setAttribute('data-spacing', space);
        el.setAttribute('data-skew', skew);
        el.setAttribute('data-color', color);
        el.style.color = color;
        state.elements[el.id].r = rotate;
        el.innerHTML = buildSignatureHTML({ text1: text, font: font, size: size, spacing: space, skew: skew });
      }
      updateElementTransform(el.id);
    }

    function deleteActiveElement() {
      if (!state.activeId) return;
      if (state.activeId === 'sealCircle' || state.activeId === 'dragSign') {
        alert('核心公章和签名不可删除');
        return;
      }
      const el = document.getElementById(state.activeId);
      if (el) el.remove();
      delete state.elements[state.activeId];
      selectElement(null);
    }

    function renderBars(containerId, widths = [], margins = []) {
      const container = document.getElementById(containerId);
      if (!container) return;
      container.innerHTML = '';
      for (let i = 0; i < widths.length; i++) {
        const bar = document.createElement('div');
        bar.className = 'bar';
        bar.style.width = `${widths[i] || 1}px`;
        bar.style.marginRight = `${margins[i] || 1}px`;
        container.appendChild(bar);
      }
    }

    function applyRandomData(data) {
      renderBars('a5BarcodeBars', data.a5_bar_widths, data.a5_bar_margins);
      renderBars('diagBarcodeBars', data.diag_bar_widths, data.diag_bar_margins);

      const medInput = document.getElementById('medCodeInput');
      if (medInput) medInput.value = data.med_code || '';
      const pNum = document.getElementById('prescNumInput');
      if (pNum) pNum.value = data.prescription_no || '';
      const dInp = document.getElementById('dateInput');
      if (dInp) dInp.value = data.date_input || '';

      const dOut = document.getElementById('diagOutpatientNum');
      if (dOut) dOut.innerText = data.outpatient_no || '';
      const dD1 = document.getElementById('diagDate1');
      if (dD1) dD1.innerText = data.diag_date1 || '';
      const dD2 = document.getElementById('diagDate2');
      if (dD2) dD2.innerText = data.diag_date2 || '';
    }

    function applyPrescriptionResult(result) {
      const rxArea = document.getElementById('a5Rx');
      if (!rxArea) return;
      rxArea.value = (result || '').trim();
      syncContent('rx', rxArea);
    }

    function syncHospitalName(source) {
      const hA5 = document.getElementById('headerHospitalNameA5');
      const hDiag = document.getElementById('diagHospitalName');
      const cSeal = document.getElementById('sealTopText');
      let v = '';
      if ((source === 'a5' || source === 'header') && hA5) v = hA5.value;
      else if (source === 'control' && cSeal) v = cSeal.value;
      else if (source === 'diag' && hDiag) v = hDiag.value;

      if (hA5) hA5.value = v;
      if (hDiag) hDiag.value = v;
      if (cSeal) cSeal.value = v;
      const circle = document.getElementById('sealCircle');
      if (circle) {
        circle.setAttribute('data-text1', v);
        circle.innerHTML = buildCircleSealSVG(circle.id, v, circle.getAttribute('data-text2'), circle.getAttribute('data-grunge'));
      }
    }

    function syncContent(type, sourceEl) {
      const val = sourceEl.tagName === 'SPAN' ? sourceEl.innerText : sourceEl.value;
      if (type === 'diag') {
        const a5 = document.getElementById('a5Diag');
        const a4 = document.getElementById('a4Diag');
        if (a5 && sourceEl !== a5) a5.value = val;
        if (a4 && sourceEl !== a4) a4.innerText = val;
      } else if (type === 'rx') {
        const a5 = document.getElementById('a5Rx');
        const a4 = document.getElementById('a4Rx');
        if (a5 && sourceEl !== a5) a5.value = val;
        if (a4 && sourceEl !== a4) a4.value = val;
      }
      resizeTextareas();
    }

    function resizeTextareas() {
      document.querySelectorAll('#pgFinalRoot .diag-textarea').forEach(el => {
        if (el.offsetParent !== null) {
          el.style.height = 'auto';
          el.style.height = el.scrollHeight + 'px';
        }
      });
    }

    function toggleMode() {
      const pLayout = document.getElementById('layout-prescription');
      const dLayout = document.getElementById('layout-diagnosis');
      const pageStyle = document.getElementById('pageStyle');
      const paper = document.getElementById('paperArea');
      const toggleBtn = document.getElementById('modeToggleBtn');

      if (state.mode === 'prescription') {
        state.mode = 'diagnosis';
        if (pLayout) pLayout.style.display = 'none';
        if (dLayout) dLayout.style.display = 'flex';
        if (pageStyle) pageStyle.innerHTML = PAPER_CONFIG.diagnosis.page;
        if (paper) {
          paper.style.width = `${PAPER_CONFIG.diagnosis.widthMm}mm`;
          paper.style.height = `${PAPER_CONFIG.diagnosis.heightMm}mm`;
        }
        if (toggleBtn) toggleBtn.innerText = PAPER_CONFIG.diagnosis.toggleText;
      } else {
        state.mode = 'prescription';
        if (pLayout) pLayout.style.display = 'flex';
        if (dLayout) dLayout.style.display = 'none';
        if (pageStyle) pageStyle.innerHTML = PAPER_CONFIG.prescription.page;
        if (paper) {
          paper.style.width = `${PAPER_CONFIG.prescription.widthMm}mm`;
          paper.style.height = `${PAPER_CONFIG.prescription.heightMm}mm`;
        }
        if (toggleBtn) toggleBtn.innerText = PAPER_CONFIG.prescription.toggleText;
      }

      const paperSize = getPaperPixelSize(state.mode);
      const paperWidth = paperSize.width;
      const paperHeight = paperSize.height;
      if (state.elements.sealCircle) {
        state.elements.sealCircle.x = state.mode === 'diagnosis' ? (paperWidth / 2 - 85) : (paperWidth - 210);
        state.elements.sealCircle.y = state.mode === 'diagnosis' ? (paperHeight - 180) : (paperHeight - 210);
        updateElementTransform('sealCircle');
      }
      if (state.elements.dragSign) {
        state.elements.dragSign.x = state.mode === 'diagnosis' ? (paperWidth - 220) : (paperWidth - 200);
        state.elements.dragSign.y = state.mode === 'diagnosis' ? (paperHeight - 110) : (paperHeight - 110);
        updateElementTransform('dragSign');
      }
      setTimeout(resizeTextareas, 10);
    }

    function bindEvents() {
      if (DOM.paper) {
        DOM.paper.addEventListener('mousedown', (e) => {
          const el = e.target.closest('.draggable-item');
          if (!el) {
            selectElement(null);
            return;
          }
          selectElement(el);
          const elState = state.elements[el.id];
          if (!elState) return;
          drag.isDragging = true;
          drag.startX = e.clientX;
          drag.startY = e.clientY;
          drag.initX = elState.x;
          drag.initY = elState.y;
          e.stopPropagation();
        });

        DOM.paper.addEventListener('touchstart', (e) => {
          const el = e.target.closest('.draggable-item');
          if (!el) {
            selectElement(null);
            return;
          }
          const touch = e.touches && e.touches[0];
          if (!touch) return;
          selectElement(el);
          const elState = state.elements[el.id];
          if (!elState) return;
          drag.isDragging = true;
          drag.startX = touch.clientX;
          drag.startY = touch.clientY;
          drag.initX = elState.x;
          drag.initY = elState.y;
          e.preventDefault();
          e.stopPropagation();
        }, { passive: false });
      }

      window.addEventListener('mousemove', (e) => {
        if (!drag.isDragging || !state.activeId) return;
        const dx = e.clientX - drag.startX;
        const dy = e.clientY - drag.startY;
        if (state.elements[state.activeId]) {
          state.elements[state.activeId].x = drag.initX + dx;
          state.elements[state.activeId].y = drag.initY + dy;
          updateElementTransform(state.activeId);
        }
      });

      window.addEventListener('mouseup', () => {
        drag.isDragging = false;
      });

      window.addEventListener('touchmove', (e) => {
        if (!drag.isDragging || !state.activeId) return;
        const touch = e.touches && e.touches[0];
        if (!touch) return;
        const dx = touch.clientX - drag.startX;
        const dy = touch.clientY - drag.startY;
        if (state.elements[state.activeId]) {
          state.elements[state.activeId].x = drag.initX + dx;
          state.elements[state.activeId].y = drag.initY + dy;
          updateElementTransform(state.activeId);
        }
        e.preventDefault();
      }, { passive: false });

      window.addEventListener('touchend', () => {
        drag.isDragging = false;
      });

      if (DOM.canvas) {
        DOM.canvas.addEventListener('mousedown', (e) => {
          if (e.button !== 0) return;
          if (e.target.closest('.draggable-item')) return;
          if (e.target.closest('input, textarea, button, select, [contenteditable]')) return;
          canvasPan.isPanning = true;
          canvasPan.startX = e.clientX;
          canvasPan.startY = e.clientY;
          canvasPan.scrollLeft = DOM.canvas.scrollLeft;
          canvasPan.scrollTop = DOM.canvas.scrollTop;
          DOM.canvas.classList.add('is-panning');
        });

        window.addEventListener('mousemove', (e) => {
          if (!canvasPan.isPanning || !DOM.canvas) return;
          const dx = e.clientX - canvasPan.startX;
          const dy = e.clientY - canvasPan.startY;
          DOM.canvas.scrollLeft = canvasPan.scrollLeft - dx;
          DOM.canvas.scrollTop = canvasPan.scrollTop - dy;
        });

        window.addEventListener('mouseup', () => {
          if (!DOM.canvas) return;
          canvasPan.isPanning = false;
          DOM.canvas.classList.remove('is-panning');
        });

        DOM.canvas.addEventListener('touchstart', (e) => {
          if (e.target.closest('.draggable-item')) return;
          if (e.target.closest('input, textarea, button, select, [contenteditable]')) return;
          const touch = e.touches && e.touches[0];
          if (!touch) return;
          canvasPan.isPanning = true;
          canvasPan.startX = touch.clientX;
          canvasPan.startY = touch.clientY;
          canvasPan.scrollLeft = DOM.canvas.scrollLeft;
          canvasPan.scrollTop = DOM.canvas.scrollTop;
          DOM.canvas.classList.add('is-panning');
        }, { passive: true });

        window.addEventListener('touchmove', (e) => {
          if (!canvasPan.isPanning || !DOM.canvas || drag.isDragging) return;
          const touch = e.touches && e.touches[0];
          if (!touch) return;
          const dx = touch.clientX - canvasPan.startX;
          const dy = touch.clientY - canvasPan.startY;
          DOM.canvas.scrollLeft = canvasPan.scrollLeft - dx;
          DOM.canvas.scrollTop = canvasPan.scrollTop - dy;
          e.preventDefault();
        }, { passive: false });

        window.addEventListener('touchend', () => {
          if (!DOM.canvas) return;
          canvasPan.isPanning = false;
          DOM.canvas.classList.remove('is-panning');
        });
      }
    }

    function exportState() {
      const formValues = {};
      document.querySelectorAll('#pgFinalRoot input, #pgFinalRoot textarea').forEach(el => {
        if (el.id) formValues[el.id] = el.value;
      });

      const editableValues = [];
      document.querySelectorAll('#pgFinalRoot .diag-editable').forEach((el, idx) => {
        editableValues.push({ idx, text: el.innerText });
      });

      return {
        mode: state.mode,
        currentDifangIdx,
        formValues,
        editableValues,
        items: extractElementItems(),
        elements: JSON.parse(JSON.stringify(state.elements || {}))
      };
    }

    function importState(payload) {
      if (!payload || typeof payload !== 'object') return;

      const nextMode = payload.mode === 'diagnosis' ? 'diagnosis' : 'prescription';
      if (state.mode !== nextMode) {
        toggleMode();
      }

      if (typeof payload.currentDifangIdx === 'number') {
        currentDifangIdx = Math.max(0, Math.min(difangTypes.length - 1, payload.currentDifangIdx));
        const difangEl = document.getElementById('difangBox');
        if (difangEl) difangEl.innerText = difangTypes[currentDifangIdx];
      }

      const formValues = payload.formValues || {};
      Object.keys(formValues).forEach(id => {
        const el = document.getElementById(id);
        if (el && ('value' in el)) el.value = formValues[id] || '';
      });

      const editableValues = Array.isArray(payload.editableValues) ? payload.editableValues : [];
      const editables = document.querySelectorAll('#pgFinalRoot .diag-editable');
      editableValues.forEach(item => {
        const target = editables[item.idx];
        if (target) target.innerText = item.text || '';
      });

      if (DOM.container) {
        DOM.container.textContent = '';
      }
      state.elements = {};
      const items = Array.isArray(payload.items) ? payload.items : buildItemsFromLegacyPayload(payload);
      items.forEach(item => {
        if (!item || typeof item !== 'object') return;
        addDraggableElement(item.type, item);
      });
      state.activeId = null;
      selectElement(null);
      resizeTextareas();
    }

    function loadScript(src) {
      return new Promise((resolve, reject) => {
        const existing = document.querySelector(`script[src="${src}"]`);
        if (existing) {
          resolve();
          return;
        }
        const script = document.createElement('script');
        script.src = src;
        script.onload = () => resolve();
        script.onerror = () => reject(new Error(`加载脚本失败: ${src}`));
        document.head.appendChild(script);
      });
    }

    async function capturePaperImage() {
      await loadScript('https://cdn.jsdelivr.net/npm/html2canvas@1.4.1/dist/html2canvas.min.js');
      const target = document.getElementById('paperArea');
      if (!target) throw new Error('找不到纸张区域');

      const prevActiveId = state.activeId;
      const prevActiveEl = prevActiveId ? document.getElementById(prevActiveId) : null;
      if (prevActiveEl) {
        selectElement(null);
      }

      const rect = target.getBoundingClientRect();
      const exportWidth = Math.max(1, Math.round(rect.width));
      const exportHeight = Math.max(1, Math.round(rect.height));
      const exportScale = Math.max(2, Math.min(3, window.devicePixelRatio || 1));

      setScreenWatermarkVisible(false);
      try {
        const canvas = await window.html2canvas(target, {
          width: exportWidth,
          height: exportHeight,
          scale: exportScale,
          windowWidth: exportWidth,
          windowHeight: exportHeight,
          scrollX: 0,
          scrollY: 0,
          onclone: (docClone) => {
            const clonedPaper = docClone.getElementById('paperArea');
            if (!clonedPaper) return;

            const compactStyle = docClone.createElement('style');
            compactStyle.textContent = `
              #pgFinalRoot .hospital-input { line-height: 1.08 !important; }
              #pgFinalRoot .diag-info-grid { line-height: 1.42 !important; }
              #pgFinalRoot .diag-text-block { line-height: 1.5 !important; }
              #pgFinalRoot .diag-textarea { line-height: 1.5 !important; }
              #pgFinalRoot .diag-footer-area { line-height: 1.42 !important; }
            `;
            if (docClone.head) {
              docClone.head.appendChild(compactStyle);
            }

            clonedPaper.querySelectorAll('div[style*="line-height:1.6"]').forEach((el) => {
              el.style.lineHeight = '1.25';
            });
            clonedPaper.querySelectorAll('textarea[style*="line-height:1.8"]').forEach((el) => {
              el.style.lineHeight = '1.5';
            });

            const shrinkLineHeight = (lineHeightValue, fallbackValue) => {
              if (!lineHeightValue || lineHeightValue === 'normal') return fallbackValue;
              const value = String(lineHeightValue).trim();
              const pxMatch = value.match(/^([0-9.]+)px$/);
              if (pxMatch) {
                const px = parseFloat(pxMatch[1]);
                return `${Math.max(1, px * 0.88).toFixed(2)}px`;
              }
              const unitless = Number(value);
              if (Number.isFinite(unitless) && unitless > 0) {
                return `${(unitless * 0.88).toFixed(2)}`;
              }
              return value;
            };

            clonedPaper.querySelectorAll('input').forEach((inputEl) => {
              const inputType = (inputEl.getAttribute('type') || 'text').toLowerCase();
              if (['hidden', 'checkbox', 'radio', 'button', 'submit', 'reset', 'file', 'password'].includes(inputType)) {
                return;
              }

              const computed = docClone.defaultView ? docClone.defaultView.getComputedStyle(inputEl) : null;
              const textNode = docClone.createElement('span');
              textNode.textContent = inputEl.value || inputEl.getAttribute('value') || '';
              textNode.style.display = computed ? computed.display : 'inline-block';
              textNode.style.boxSizing = 'border-box';
              textNode.style.width = computed ? computed.width : '100%';
              textNode.style.height = 'auto';
              textNode.style.minHeight = computed ? computed.height : '1em';
              textNode.style.marginTop = computed ? computed.marginTop : '0';
              textNode.style.marginBottom = computed ? computed.marginBottom : '0';
              textNode.style.marginLeft = computed ? computed.marginLeft : '0';
              textNode.style.marginRight = computed ? computed.marginRight : '0';
              textNode.style.paddingTop = computed ? computed.paddingTop : '0';
              textNode.style.paddingRight = computed ? computed.paddingRight : '0';
              textNode.style.paddingBottom = computed ? computed.paddingBottom : '0';
              textNode.style.paddingLeft = computed ? computed.paddingLeft : '0';
              textNode.style.fontFamily = computed ? computed.fontFamily : 'inherit';
              textNode.style.fontSize = computed ? computed.fontSize : '24px';
              textNode.style.fontWeight = computed ? computed.fontWeight : 'normal';
              textNode.style.letterSpacing = computed ? computed.letterSpacing : 'normal';
              textNode.style.lineHeight = shrinkLineHeight(
                computed ? computed.lineHeight : '',
                '1.08'
              );
              textNode.style.textAlign = computed ? computed.textAlign : 'left';
              textNode.style.verticalAlign = 'baseline';
              textNode.style.color = computed ? computed.color : '#000';
              textNode.style.whiteSpace = 'pre';
              textNode.style.overflow = 'visible';
              textNode.style.background = 'transparent';
              textNode.style.border = 'none';

              inputEl.replaceWith(textNode);
            });

            clonedPaper.querySelectorAll('textarea').forEach((textareaEl) => {
              const computed = docClone.defaultView ? docClone.defaultView.getComputedStyle(textareaEl) : null;
              const textNode = docClone.createElement('div');
              textNode.textContent = textareaEl.value || '';
              textNode.style.display = 'block';
              textNode.style.boxSizing = 'border-box';
              textNode.style.width = computed ? computed.width : '100%';
              textNode.style.minHeight = computed ? computed.height : '1em';
              textNode.style.marginTop = computed ? computed.marginTop : '0';
              textNode.style.marginBottom = computed ? computed.marginBottom : '0';
              textNode.style.marginLeft = computed ? computed.marginLeft : '0';
              textNode.style.marginRight = computed ? computed.marginRight : '0';
              textNode.style.paddingTop = computed ? computed.paddingTop : '0';
              textNode.style.paddingRight = computed ? computed.paddingRight : '0';
              textNode.style.paddingBottom = computed ? computed.paddingBottom : '0';
              textNode.style.paddingLeft = computed ? computed.paddingLeft : '0';
              textNode.style.fontFamily = computed ? computed.fontFamily : 'inherit';
              textNode.style.fontSize = computed ? computed.fontSize : '16px';
              textNode.style.fontWeight = computed ? computed.fontWeight : 'normal';
              textNode.style.letterSpacing = computed ? computed.letterSpacing : 'normal';
              textNode.style.lineHeight = shrinkLineHeight(
                computed ? computed.lineHeight : '',
                '1.5'
              );
              textNode.style.textAlign = computed ? computed.textAlign : 'left';
              textNode.style.color = computed ? computed.color : '#000';
              textNode.style.whiteSpace = 'pre-wrap';
              textNode.style.wordBreak = 'break-word';
              textNode.style.overflowWrap = 'anywhere';
              textNode.style.background = 'transparent';
              textNode.style.border = 'none';

              textareaEl.replaceWith(textNode);
            });
          },
          useCORS: true,
          backgroundColor: '#ffffff'
        });
        return canvas.toDataURL('image/png');
      } finally {
        setScreenWatermarkVisible(true);
        if (prevActiveEl) {
          selectElement(prevActiveEl);
        }
      }
    }

    return {
      init: function () {
        bindEvents();
        addDraggableElement('circle', { id: 'sealCircle', text1: 'XXX中心医院', text2: '处方专用章', grunge: 60, rotate: -2, x: 590, y: 350 });
        addDraggableElement('sign', { id: 'dragSign', text1: '王医生', color: '#1a2233', rotate: -5, skew: -25, size: 50, font: "'Liu Jian Mao Cao', cursive", spacing: -8, x: 600, y: 450 });
        syncHospitalName('control');
        document.querySelectorAll('#pgFinalRoot .diag-textarea').forEach(el => {
          el.addEventListener('input', resizeTextareas);
        });
        selectElement(null);
        setTimeout(resizeTextareas, 100);
      },
      addRectSeal: function (txt) { addDraggableElement('rect', { text1: txt, grunge: 30, rotate: 2 }); },
      addSignature: function (txt) { addDraggableElement('sign', { text1: txt, rotate: -5, skew: -25, size: 50, font: "'Liu Jian Mao Cao', cursive", spacing: -8, color: '#1a2233' }); },
      updateActiveElement: updateActiveElement,
      deleteActiveElement: deleteActiveElement,
      toggleMode: toggleMode,
      toggleDifang: toggleDifang,
      applyRandomData: applyRandomData,
      syncHospitalName: syncHospitalName,
      syncContent: syncContent,
      applyPrescriptionResult: applyPrescriptionResult,
      exportState: exportState,
      importState: importState,
      capturePaperImage: capturePaperImage,
      updateGlobalSealColor: function () {
        const color = document.getElementById('globalSealColor').value;
        document.querySelectorAll('#pgFinalRoot .draggable-item[data-type="circle"], #pgFinalRoot .draggable-item[data-type="rect"]').forEach(el => {
          el.setAttribute('data-color', color);
          el.style.color = color;
        });
      }
    };
  })();

  function initPresentationBridge() {
    App.init();
    window.PresentationBridge = {
      exportState: () => App.exportState(),
      importState: (payload) => App.importState(payload),
      applyPrescriptionResult: (result) => App.applyPrescriptionResult(result),
      applyRandomData: (data) => App.applyRandomData(data),
      capturePaperImage: () => App.capturePaperImage()
    };
  }

  if (document.readyState === 'loading') {
    window.addEventListener('DOMContentLoaded', initPresentationBridge, { once: true });
  } else {
    initPresentationBridge();
  }
