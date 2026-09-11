const state = {
  connected: false,
  status: 'sin conexión',
  phase: 'OFFLINE',
  info: null,
  models: [],
  datasets: [],
  jobs: [],
  executors: [],
  selectedModels: [],
  selectedDatasetId: null,
  selectedWorkflow: null,
  uploadedDataset: null,
  datasetValidation: null,
  availableWorkflows: [],
  events: [],
  evidenceEntries: [],
  plasticity: {
    available: false,
    source_jobs: 0,
    elo_leaderboard: [],
    elo_entities: [],
    routing_decisions: [],
    notes: []
  }
};

const els = {
  systemStatus: document.getElementById('systemStatus'),
  runtimeBadge: document.getElementById('runtimeBadge'),
  statusDot: document.getElementById('statusDot'),
  metricAuthority: document.getElementById('metricAuthority'),
  metricEvidence: document.getElementById('metricEvidence'),
  metricCapacity: document.getElementById('metricCapacity'),
  metricRuntime: document.getElementById('metricRuntime'),
  phasePill: document.getElementById('phasePill'),
  timelineList: document.getElementById('timelineList'),
  summaryBox: document.getElementById('summaryBox'),
  eventLog: document.getElementById('eventLog'),
  evidenceLog: document.getElementById('evidenceLog'),
  receiptList: document.getElementById('receiptList'),
  pipelineOverview: document.getElementById('pipelineOverview'),
  moduleGrid: document.getElementById('moduleGrid'),
  plasticityPanel: document.getElementById('plasticityPanel'),
  goalInput: document.getElementById('goalInput'),
  resetBtn: document.getElementById('resetBtn'),
  refreshBtn: document.getElementById('refreshBtn'),
  scanBtn: document.getElementById('scanBtn'),
  buildPipelineBtn: document.getElementById('buildPipelineBtn'),
  runCycleBtn: document.getElementById('runCycleBtn'),
  modelSelector: document.getElementById('modelSelector'),
  datasetSelector: document.getElementById('datasetSelector'),
  datasetUpload: document.getElementById('datasetUpload'),
  datasetValidation: document.getElementById('datasetValidation'),
  workflowSelector: document.getElementById('workflowSelector'),
  executionSummary: document.getElementById('executionSummary'),
  runSelectedWorkflowBtn: document.getElementById('runSelectedWorkflowBtn')
};

const workflowCatalog = [
  {
    id: 'behavioral_discovery',
    title: 'Descubrimiento conductual comparado',
    endpoint: 'behavioral-discovery',
    requiresModels: { min: 2, exact: null },
    requiresDataset: 'benchmark-json',
    category: 'comparativo'
  },
  {
    id: 'probe_runtime',
    title: 'Comprobar runtime HF',
    endpoint: 'direct',
    requiresModels: { min: 1, exact: 1 },
    requiresDataset: 'none',
    category: 'runtime'
  },
  {
    id: 'behavioral_evaluation',
    title: 'Evaluación conductual',
    endpoint: 'direct',
    requiresModels: { min: 1, exact: 1 },
    requiresDataset: 'any',
    category: 'evaluación'
  },
  {
    id: 'extract_capability',
    title: 'Extracción de capacidad',
    endpoint: 'direct',
    requiresModels: { min: 1, exact: 1 },
    requiresDataset: 'none',
    category: 'capacidad'
  },
  {
    id: 'deep_instrumentation',
    title: 'Instrumentación profunda (NNsight)',
    endpoint: 'direct',
    requiresModels: { min: 1, exact: 1 },
    requiresDataset: 'none',
    category: 'instrumentación'
  },
  {
    id: 'sparse_autoencoder_analysis',
    title: 'Análisis SAE',
    endpoint: 'direct',
    requiresModels: { min: 1, exact: 1 },
    requiresDataset: 'none',
    category: 'instrumentación'
  },
  {
    id: 'counterfactual_analysis',
    title: 'Análisis contrafactual',
    endpoint: 'direct',
    requiresModels: { min: 1, exact: 1 },
    requiresDataset: 'none',
    category: 'análisis'
  },
  {
    id: 'generate_behavioral_dataset',
    title: 'Generar dataset conductual',
    endpoint: 'direct',
    requiresModels: { min: 1, exact: 1 },
    requiresDataset: 'none',
    category: 'dataset'
  },
  {
    id: 'calibrate_alignment',
    title: 'Calibrar alineamiento A→B',
    endpoint: 'direct',
    requiresModels: { min: 2, exact: 2 },
    requiresDataset: 'none',
    category: 'alineamiento'
  },
  {
    id: 'activation_transfer_experiment',
    title: 'Transferencia por activation steering',
    endpoint: 'direct',
    requiresModels: { min: 2, exact: 2 },
    requiresDataset: 'any',
    category: 'transferencia'
  }
];

function safeText(value) {
  return String(value ?? '').replace(/[&<>"']/g, (char) => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;'
  }[char]));
}

function modelKey(model) {
  return model && model.model_id ? String(model.model_id) : null;
}

function datasetId(dataset) {
  if (!dataset) return null;
  return dataset.content_sha256 || dataset.id || null;
}

function datasetFormatLabel(format) {
  return ({ json: 'JSON', jsonl: 'JSONL', csv: 'CSV', text: 'Texto' })[format] || String(format || 'archivo');
}

function engineLabel(info) {
  return (info && info.engine) ? String(info.engine) : 'tidex';
}

function hfWorkerLabel(info) {
  const path = info && (info.hf_python || info.configured_hf_python);
  if (!path) return 'sin worker HF';
  const parts = String(path).split('/').filter(Boolean);
  const venv = parts.find((part) => part === 'tidex-mechinterp') || parts[parts.length - 2];
  return venv ? `worker HF ${venv}` : 'worker HF';
}

async function fetchJson(url, options = {}) {
  const response = await fetch(url, { cache: 'no-store', ...options });
  const text = await response.text();
  let parsed = text;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    parsed = text;
  }
  if (!response.ok) {
    const message = parsed && parsed.error ? parsed.error : (text || `${response.status} ${response.statusText}`);
    throw new Error(message);
  }
  return parsed;
}

function addEvent(title, detail) {
  state.events.unshift({ title, detail, stamp: new Date().toLocaleTimeString() });
  if (state.events.length > 12) state.events.pop();
  renderEventLog();
}

function addEvidence(title, detail) {
  state.evidenceEntries.unshift({ title, detail, stamp: new Date().toLocaleTimeString() });
  if (state.evidenceEntries.length > 8) state.evidenceEntries.pop();
  renderEvidence();
}

function renderEventLog() {
  if (!els.eventLog) return;
  els.eventLog.innerHTML = state.events.length
    ? state.events.map((event) => `
        <div class="log-entry">
          <strong>${safeText(event.title)}</strong>
          <small>${safeText(event.detail)} · ${safeText(event.stamp)}</small>
        </div>
      `).join('')
    : '<div class="log-entry"><strong>Sin eventos</strong><small>La actividad de la API aparecerá aquí.</small></div>';
}

function renderEvidence() {
  if (!els.evidenceLog) return;
  els.evidenceLog.innerHTML = state.evidenceEntries.length
    ? state.evidenceEntries.map((entry) => `
        <div class="log-entry">
          <strong>${safeText(entry.title)}</strong>
          <small>${safeText(entry.detail)} · ${safeText(entry.stamp)}</small>
        </div>
      `).join('')
    : '<div class="log-entry"><strong>Sin evidencia de jobs</strong><small>Un receipt aparece cuando TIDE-X completa un trabajo.</small></div>';
}

function renderReceipts() {
  if (!els.receiptList) return;
  const receipts = (state.jobs || []).map((job) => ({
    name: job.job_id || 'job',
    status: job.state || 'unknown',
    hash: (job.evidence_receipt && job.evidence_receipt.evidence_sha256) || 'sin receipt'
  }));
  els.receiptList.innerHTML = receipts.length
    ? receipts.map((receipt) => `
        <div class="receipt-item">
          <strong>${safeText(String(receipt.name).slice(0, 16))}…</strong>
          <small>${safeText(receipt.status)}</small>
          <div class="hash">${safeText(receipt.hash)}</div>
        </div>
      `).join('')
    : '<div class="receipt-item"><strong>Sin jobs</strong><small>vacío</small><div class="hash">—</div></div>';
}

function renderPipeline() {
  const source = (state.executors || []).slice(0, 8);
  if (!els.pipelineOverview) return;
  if (!source.length) {
    els.pipelineOverview.innerHTML = '<div class="summary-box">Sin catálogo de ejecutores. Conecta la API o abre /operator.</div>';
    return;
  }
  els.pipelineOverview.innerHTML = `
    <div class="pipeline-grid">
      ${source.map((pipe) => {
        const stateClass = pipe.state === 'operational' ? 'available' : pipe.state === 'operational_needs_workflow' ? 'manual' : 'blocked';
        return `
          <article class="pipeline-step ${stateClass}">
            <div class="head">
              <strong>${safeText(pipe.title || pipe.executor_id)}</strong>
              <span class="chip ${stateClass === 'available' ? 'op' : stateClass === 'manual' ? 'need' : 'arch'}">${safeText(pipe.state || 'registrado')}</span>
            </div>
            <div class="why">${safeText(pipe.notes || pipe.effect_class || pipe.executor_id)}</div>
          </article>
        `;
      }).join('')}
    </div>
  `;
}

function renderModuleGrid() {
  if (!els.moduleGrid) return;
  const modules = (state.executors || []).slice(0, 12);
  if (!modules.length) {
    els.moduleGrid.innerHTML = '<div class="summary-box">El catálogo de autoridades llega de /api/executors. No hay datos locales de relleno.</div>';
    return;
  }
  els.moduleGrid.innerHTML = modules.map((executor) => {
    const kind = executor.state === 'operational' ? 'op' : executor.state === 'operational_needs_workflow' ? 'need' : executor.state === 'experimental_candidate_only' ? 'exp' : 'arch';
    return `
      <article class="module-card ${kind}">
        <div class="module-header">
          <strong>${safeText(executor.title || executor.executor_id)}</strong>
          <span class="chip ${kind}">${safeText(executor.effect_class || executor.authority || 'ejecutor')}</span>
        </div>
        <p>${safeText(executor.notes || executor.module_path || '')}</p>
        <small>${safeText(executor.state || '')}${executor.production_authority ? ' · production_authority' : ''}</small>
      </article>
    `;
  }).join('');
}

function renderPlasticity(activeTab = document.querySelector('.tab-btn.active')?.dataset.tab || 'state') {
  const current = state.plasticity || { available: false, source_jobs: 0, elo_leaderboard: [], elo_entities: [], routing_decisions: [], notes: [] };
  const entities = (current.elo_entities || []).filter((item) => Number(item.comparisons) > 0);
  const routes = current.routing_decisions || [];
  const signal = Boolean(current.available) && (entities.length > 0 || routes.length > 0);
  const notes = (current.notes && current.notes.length)
    ? current.notes.join(' · ')
    : (signal
      ? 'Señal comparativa medida a partir de scores reales.'
      : 'Sin señal comparativa. El motor compilado no es un ranking.');
  const tabs = {
    state: `
      <div class="metric-grid compact">
        <div class="metric-card mini ${signal ? 'ok' : 'warn'}"><span class="label">Señal comparativa</span><strong>${signal ? 'SÍ' : 'NO'}</strong></div>
        <div class="metric-card mini"><span class="label">Jobs fuente</span><strong>${current.source_jobs || 0}</strong></div>
        <div class="metric-card mini"><span class="label">ELO</span><strong>${entities.length}</strong></div>
        <div class="metric-card mini"><span class="label">Rutas</span><strong>${routes.length}</strong></div>
      </div>
      <div class="summary-box">${safeText(notes)}</div>
    `,
    experiments: `
      <div class="result-list">
        ${(state.jobs || []).filter((job) => {
          const op = job.operation || '';
          return op === 'behavioral_evaluation' || op === 'behavioral_discovery' || op === 'activation_transfer_experiment';
        }).slice(0, 8).map((job) => `
          <div class="result-row"><strong>${safeText(String(job.job_id || '').slice(0, 12))}</strong><span>${safeText(job.operation || 'job')}</span><span class="${job.state === 'completed' ? 'good' : job.state === 'failed' ? 'bad' : 'running'}">${safeText(job.state || 'unknown')}</span></div>
        `).join('') || '<div class="summary-box">Sin experimentos comparativos. probe_runtime no cuenta como evidencia de plasticidad.</div>'}
      </div>
    `,
    elo: `
      <div class="result-list">
        ${entities.map((item) => `
          <div class="result-row"><strong>${safeText(item.entity)}</strong><span>${Number(item.comparisons)} comparación(es)</span><span>${Number(item.rating).toFixed(2)}</span></div>
        `).join('') || '<div class="summary-box">Sin ranking ELO. Hace falta un hueco de score medido entre al menos dos modelos en el mismo benchmark.</div>'}
      </div>
    `,
    routes: `
      <div class="route-list">
        ${routes.map((route) => `
          <article class="route-card">
            <strong>${safeText(route.capability || 'capacidad')}</strong>
            <div class="route-path">${safeText(route.target_model || '')} · medido ${Number(route.measured_score || 0).toFixed(3)} · ruta ${Number(route.routing_score || 0).toFixed(3)}</div>
          </article>
        `).join('') || '<div class="summary-box">Sin rutas. Un empate o una sola evaluación no elige modelo.</div>'}
      </div>
    `,
    learning: `
      <div class="summary-box">El catálogo lista mecanismos. Eso no prueba que hayan corrido ni que exista un ranking.</div>
      <div class="learning-grid">
        ${(state.executors || []).filter((entry) => {
          const id = entry.executor_id || '';
          return id.startsWith('plasticity.') || id.startsWith('learning.') || id === 'numerical.evolve' || id === 'procedural.memory';
        }).map((entry) => `
          <div class="learning-card"><strong>${safeText(entry.title || entry.executor_id)}</strong><small>${safeText(entry.state || '')}</small></div>
        `).join('') || '<div class="summary-box">Sin ejecutores de aprendizaje en el catálogo.</div>'}
      </div>
    `
  };
  if (!els.plasticityPanel) return;
  els.plasticityPanel.innerHTML = tabs[activeTab] || tabs.state;
}

function refreshMetrics() {
  const completed = (state.jobs || []).filter((job) => job.state === 'completed').length;
  const withReceipt = (state.jobs || []).filter((job) => job.evidence_receipt && job.evidence_receipt.evidence_sha256).length;
  els.systemStatus.textContent = state.connected ? 'CONECTADO' : 'SIN CONEXIÓN';
  els.statusDot.className = `status-dot${state.connected ? ' live' : ''}`;
  els.runtimeBadge.textContent = state.connected
    ? (state.info && state.info.operator_home ? String(state.info.operator_home) : 'TIDE-X')
    : 'API no disponible';
  els.metricAuthority.textContent = state.connected ? 'sí' : 'no';
  els.metricEvidence.textContent = `${withReceipt}/${(state.jobs || []).length}`;
  els.metricCapacity.textContent = String((state.models || []).length);
  els.metricRuntime.textContent = state.connected ? engineLabel(state.info) : 'offline';
  els.phasePill.textContent = state.phase;
  els.summaryBox.textContent = state.connected
    ? `Motor ${engineLabel(state.info)} en ${state.info && state.info.operator_home ? state.info.operator_home : 'TIDEX_HOME'}. ${hfWorkerLabel(state.info)}. Modelos: ${(state.models || []).length}. Jobs: ${(state.jobs || []).length} (${completed} completados). AdapterBank sigue siendo la autoridad de activación.`
    : 'No hay conexión con http://127.0.0.1:8793. Arranca `./tidex serve`. Esta UI no rellena métricas falsas.';
  const steps = els.timelineList.querySelectorAll('li');
  steps.forEach((item, index) => {
    item.classList.remove('done', 'doing');
    if (state.connected && index === 0) item.classList.add('done');
    if (state.connected && (state.models || []).length && index === 1) item.classList.add('done');
    if (state.connected && state.selectedWorkflow && index === 2) item.classList.add('doing');
    if (withReceipt && index === 3) item.classList.add('done');
  });
}

function getSelectedDataset() {
  if (state.uploadedDataset && state.selectedDatasetId === state.uploadedDataset.id) {
    return state.uploadedDataset;
  }
  return (state.datasets || []).find((dataset) => datasetId(dataset) === state.selectedDatasetId) || null;
}

function workflowFits(workflow) {
  const count = state.selectedModels.length;
  if (workflow.requiresModels.exact != null && count !== workflow.requiresModels.exact) return false;
  if (count < workflow.requiresModels.min) return false;
  const dataset = getSelectedDataset();
  if (workflow.requiresDataset === 'benchmark-json') {
    return !!(dataset && dataset.format === 'json');
  }
  if (workflow.requiresDataset === 'any') {
    return !!dataset;
  }
  return true;
}

function computeAvailableWorkflows() {
  const workflows = workflowCatalog.filter(workflowFits);
  state.availableWorkflows = workflows;
  if (!state.selectedWorkflow || !workflows.some((item) => item.id === state.selectedWorkflow)) {
    state.selectedWorkflow = workflows[0]?.id || null;
  }
  return workflows;
}

function getWorkflowById(id) {
  return workflowCatalog.find((workflow) => workflow.id === id) || null;
}

function renderWorkflowWizard() {
  const available = computeAvailableWorkflows();
  els.modelSelector.innerHTML = (state.models.length ? state.models : [{ model_id: null, root: 'Sin modelos catalogados. Usa Escanear modelos.' }])
    .map((model) => {
      const key = modelKey(model);
      const selected = !!key && state.selectedModels.includes(key);
      const disabled = !key;
      const label = model.architecture || 'HF local';
      const path = model.root || model.model_id || '';
      return `
        <button type="button" class="option-card ${selected ? 'selected' : ''} ${disabled ? 'disabled' : ''}" data-model-key="${safeText(key || '')}" ${disabled ? 'disabled' : ''}>
          <strong>${safeText(label)}</strong>
          <small>${safeText(key ? `${key.slice(0, 16)}…` : 'sin id')}</small>
          <div class="option-meta">
            <span>${safeText(model.layout || '')}</span>
            <span>${selected ? 'seleccionado' : 'pendiente'}</span>
          </div>
          <small>${safeText(path)}</small>
        </button>
      `;
    }).join('');

  const datasets = state.datasets.length ? state.datasets : [];
  const extra = state.uploadedDataset && !datasets.some((dataset) => datasetId(dataset) === state.uploadedDataset.id)
    ? [state.uploadedDataset, ...datasets]
    : datasets;
  const displayed = extra.length ? extra : [{ name: 'Sin datasets', format: 'text', id: 'none' }];
  els.datasetSelector.innerHTML = displayed.map((dataset) => {
    const id = datasetId(dataset) || dataset.id || 'none';
    const selected = id === state.selectedDatasetId;
    const disabled = id === 'none';
    return `
      <button type="button" class="option-card ${selected ? 'selected' : ''} ${disabled ? 'disabled' : ''}" data-dataset-id="${safeText(id)}" ${disabled ? 'disabled' : ''}>
        <strong>${safeText(dataset.name || 'Dataset')}</strong>
        <div class="option-meta">
          <span>${safeText(datasetFormatLabel(dataset.format))}</span>
          <span>${dataset.generated ? 'generado' : 'catalogado'}</span>
        </div>
        <small>${safeText(dataset.content_sha256 ? `${dataset.content_sha256.slice(0, 16)}…` : 'sin SHA todavía')}</small>
      </button>
    `;
  }).join('');

  let validationMessage = 'Elige un dataset catalogado o carga un archivo. El descubrimiento conductual exige benchmark JSON.';
  let validationKind = 'neutral';
  if (state.datasetValidation) {
    validationKind = state.datasetValidation.kind;
    validationMessage = state.datasetValidation.message;
  } else if (getSelectedDataset()) {
    const dataset = getSelectedDataset();
    validationKind = dataset.format === 'json' ? 'success' : 'warning';
    validationMessage = dataset.format === 'json'
      ? 'JSON seleccionado. Si no tiene el schema de benchmark, el descubrimiento conductual fallará en el servidor.'
      : 'Dataset compatible para importación; no sirve como benchmark conductual.';
  }
  els.datasetValidation.className = `validation-box ${validationKind}`;
  els.datasetValidation.textContent = validationMessage;

  if (!available.length) {
    els.workflowSelector.innerHTML = '<div class="summary-box">Ningún workflow del enum real encaja con esta selección. probe_runtime pide 1 modelo; calibrate_alignment y transfer piden exactamente 2; el descubrimiento comparado pide ≥2 y un JSON.</div>';
  } else {
    els.workflowSelector.innerHTML = available.map((workflow) => `
      <button type="button" class="option-card ${workflow.id === state.selectedWorkflow ? 'selected' : ''}" data-workflow-id="${workflow.id}">
        <strong>${safeText(workflow.title)}</strong>
        <div class="option-meta">
          <span>${safeText(workflow.category)}</span>
          <span>${workflow.endpoint}</span>
        </div>
        <small>${safeText(workflow.id)}</small>
      </button>
    `).join('');
  }

  const workflow = getWorkflowById(state.selectedWorkflow);
  const dataset = getSelectedDataset();
  const canRun = !!(workflow && state.connected);
  els.runSelectedWorkflowBtn.disabled = !canRun;
  els.runCycleBtn.disabled = !canRun;
  els.executionSummary.innerHTML = workflow
    ? `<strong>${safeText(workflow.title)}</strong><br>
       Modelos: ${state.selectedModels.length ? safeText(state.selectedModels.map((id) => id.slice(0, 12)).join(', ')) : 'ninguno'}<br>
       Dataset: ${safeText(dataset ? (dataset.name || datasetId(dataset)) : 'ninguno')}<br>
       Endpoint: /api/workflows/${safeText(workflow.endpoint)}<br>
       El operator marcará authorizes_production=false.`
    : '<strong>Sin workflow válido.</strong>';
}

async function ensureDatasetReady() {
  const dataset = getSelectedDataset();
  if (!dataset) return null;
  if (dataset.content_sha256) return dataset.content_sha256;
  if (!dataset.content) return null;
  const imported = await fetchJson('/api/datasets/import', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      name: dataset.name,
      format: dataset.format,
      content: dataset.content,
      generated: !!dataset.generated
    })
  });
  state.datasets = [imported, ...state.datasets.filter((item) => datasetId(item) !== imported.content_sha256)];
  state.uploadedDataset = { ...imported };
  state.selectedDatasetId = imported.content_sha256;
  return imported.content_sha256;
}

async function pollJob(job) {
  if (!job || !job.job_id) return job;
  for (let i = 0; i < 120; i += 1) {
    const current = await fetchJson(`/api/jobs/${job.job_id}`);
    state.jobs = [current, ...state.jobs.filter((item) => item.job_id !== current.job_id)];
    renderReceipts();
    refreshMetrics();
    if (['completed', 'failed', 'cancelled'].includes(current.state)) {
      addEvent(`Job ${current.state}`, current.error || current.operation || current.job_id);
      if (current.evidence_receipt && current.evidence_receipt.evidence_sha256) {
        addEvidence('Receipt', current.evidence_receipt.evidence_sha256);
      }
      return current;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  throw new Error('El job sigue en curso; mira /api/jobs o /operator.');
}

async function runSelectedWorkflow() {
  if (!state.connected) {
    throw new Error('Sin API. Arranca TIDE-X con ./tidex serve.');
  }
  const workflow = getWorkflowById(state.selectedWorkflow) || computeAvailableWorkflows()[0];
  if (!workflow) {
    throw new Error('No hay un workflow compatible con la selección.');
  }
  if (!workflowFits(workflow)) {
    throw new Error(`La selección no cumple ${workflow.id}.`);
  }
  const datasetSha = workflow.requiresDataset === 'none' ? null : await ensureDatasetReady();
  if (workflow.requiresDataset !== 'none' && !datasetSha) {
    throw new Error('Este workflow necesita un dataset catalogado.');
  }

  let body;
  let url;
  if (workflow.endpoint === 'behavioral-discovery') {
    url = '/api/workflows/behavioral-discovery';
    body = {
      schema: 'cerebro.tidex.operator_behavioral_discovery/v1',
      model_ids: state.selectedModels,
      dataset_sha256: datasetSha,
      max_new_tokens: 128,
      seed: 0
    };
  } else {
    url = '/api/workflows/direct';
    body = {
      schema: 'cerebro.tidex.operator_direct_workflow/v1',
      operation: workflow.id,
      model_ids: state.selectedModels,
      dataset_sha256: datasetSha,
      parameters: {}
    };
  }

  state.phase = 'RUNNING';
  refreshMetrics();
  const job = await fetchJson(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body)
  });
  addEvent('Job lanzado', job.job_id || workflow.id);
  await pollJob(job);
  await refreshLiveData({ quiet: true });
}

async function handleDatasetUpload(event) {
  const file = event.target.files && event.target.files[0];
  if (!file) return;
  const ext = file.name.split('.').pop()?.toLowerCase() || 'txt';
  const format = ['json', 'jsonl', 'csv'].includes(ext) ? ext : 'text';
  const content = await file.text();
  let kind = 'neutral';
  let message = 'Archivo listo para importar al catálogo de TIDE-X.';
  if (format === 'json') {
    try {
      const parsed = JSON.parse(content);
      if (parsed && parsed.schema === 'cerebro.cross_model.behavioral_benchmark/v1') {
        kind = 'success';
        message = 'Benchmark conductual reconocido.';
      } else {
        kind = 'warning';
        message = 'JSON válido, pero no es el schema de benchmark conductual.';
      }
    } catch {
      kind = 'warning';
      message = 'El JSON no parsea.';
    }
  }
  state.uploadedDataset = {
    id: `upload:${Date.now()}`,
    name: file.name.replace(/\.[^/.]+$/, '') || 'dataset-importado',
    format,
    content,
    generated: false
  };
  state.selectedDatasetId = state.uploadedDataset.id;
  state.datasetValidation = { kind, message };
  renderWorkflowWizard();
}

async function scanModels() {
  if (!state.info || !state.info.default_model_scan_root) {
    throw new Error('No hay default_model_scan_root. Cataloga modelos con ./tidex models scan <ruta>.');
  }
  const models = await fetchJson('/api/models/scan', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ root: state.info.default_model_scan_root })
  });
  state.models = Array.isArray(models) ? models : [];
  addEvent('Scan', `${state.models.length} modelo(s) en ${state.info.default_model_scan_root}`);
  renderWorkflowWizard();
  refreshMetrics();
}

async function refreshLiveData({ quiet } = {}) {
  try {
    const [info, models, datasets, plasticity, executors, jobs] = await Promise.all([
      fetchJson('/api/info'),
      fetchJson('/api/models'),
      fetchJson('/api/datasets'),
      fetchJson('/api/plasticity'),
      fetchJson('/api/executors'),
      fetchJson('/api/jobs')
    ]);
    state.connected = true;
    state.phase = 'READY';
    state.status = 'conectado';
    state.info = info;
    state.models = Array.isArray(models) ? models : [];
    state.datasets = Array.isArray(datasets) ? datasets : [];
    state.plasticity = plasticity || state.plasticity;
    state.executors = Array.isArray(executors) ? executors : [];
    state.jobs = Array.isArray(jobs) ? jobs : [];
    if (!state.selectedModels.length && state.models.length) {
      state.selectedModels = [modelKey(state.models[0])].filter(Boolean);
    }
    if (!quiet) addEvent('API conectada', info.operator_home || 'TIDE-X');
    if (!state.models.length && info.default_model_scan_root) {
      try {
        await scanModels();
      } catch (error) {
        addEvent('Scan', error.message);
      }
    }
  } catch (error) {
    state.connected = false;
    state.phase = 'OFFLINE';
    state.status = 'sin conexión';
    state.info = null;
    state.models = [];
    state.datasets = [];
    state.executors = [];
    state.jobs = [];
    state.plasticity = {
      available: false,
      source_jobs: 0,
      elo_leaderboard: [],
      elo_entities: [],
      routing_decisions: [],
      notes: []
    };
    if (!quiet) addEvent('API ausente', error.message || 'sin /api/info');
  }
  renderPipeline();
  renderModuleGrid();
  renderPlasticity();
  renderEvidence();
  renderReceipts();
  renderWorkflowWizard();
  refreshMetrics();
}

function resetState() {
  state.selectedModels = [];
  state.selectedDatasetId = null;
  state.selectedWorkflow = null;
  state.uploadedDataset = null;
  state.datasetValidation = null;
  state.events = [];
  state.evidenceEntries = [];
  renderEventLog();
  renderEvidence();
  refreshLiveData();
}

document.querySelectorAll('.nav-item').forEach((button) => {
  button.addEventListener('click', () => {
    document.querySelectorAll('.nav-item').forEach((item) => item.classList.remove('active'));
    button.classList.add('active');
    document.querySelectorAll('.panel').forEach((panel) => panel.classList.remove('active'));
    const target = document.getElementById(button.dataset.panel);
    if (target) target.classList.add('active');
  });
});

document.querySelectorAll('.tab-btn').forEach((button) => {
  button.addEventListener('click', () => {
    document.querySelectorAll('.tab-btn').forEach((item) => item.classList.remove('active'));
    button.classList.add('active');
    renderPlasticity(button.dataset.tab);
  });
});

els.buildPipelineBtn.addEventListener('click', () => {
  computeAvailableWorkflows();
  renderWorkflowWizard();
  addEvent('Plan revisado', state.selectedWorkflow || 'sin workflow válido');
});
els.runCycleBtn.addEventListener('click', async () => {
  try {
    await runSelectedWorkflow();
  } catch (error) {
    addEvent('Ejecución rechazada', error.message);
  }
});
els.runSelectedWorkflowBtn.addEventListener('click', async () => {
  try {
    await runSelectedWorkflow();
  } catch (error) {
    addEvent('Ejecución rechazada', error.message);
  }
});
els.resetBtn.addEventListener('click', resetState);
els.refreshBtn.addEventListener('click', () => refreshLiveData());
els.scanBtn.addEventListener('click', async () => {
  try {
    await scanModels();
  } catch (error) {
    addEvent('Scan fallido', error.message);
  }
});
els.datasetUpload.addEventListener('change', handleDatasetUpload);

els.modelSelector.addEventListener('click', (event) => {
  const button = event.target.closest('[data-model-key]');
  if (!button || !button.dataset.modelKey) return;
  const key = button.dataset.modelKey;
  state.selectedModels = state.selectedModels.includes(key)
    ? state.selectedModels.filter((item) => item !== key)
    : [...state.selectedModels, key];
  renderWorkflowWizard();
});

els.datasetSelector.addEventListener('click', (event) => {
  const button = event.target.closest('[data-dataset-id]');
  if (!button || !button.dataset.datasetId || button.dataset.datasetId === 'none') return;
  state.selectedDatasetId = button.dataset.datasetId;
  state.datasetValidation = null;
  renderWorkflowWizard();
});

els.workflowSelector.addEventListener('click', (event) => {
  const button = event.target.closest('[data-workflow-id]');
  if (!button) return;
  state.selectedWorkflow = button.dataset.workflowId;
  renderWorkflowWizard();
});

renderEventLog();
renderEvidence();
renderReceipts();
renderWorkflowWizard();
refreshMetrics();
refreshLiveData();
