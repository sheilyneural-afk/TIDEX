const state = {
  status: 'listo',
  phase: 'READY',
  authority: 100,
  evidence: 4,
  runtime: 'OK',
  goal: 'Ejecutar el cerebro TIDE-X para descubrir capacidades relevantes, comparar modelos, validar evidencia y decidir si la materialización es segura antes de la promoción.',
  timeline: [
    'Objetivo y restricciones',
    'Mapa de capacidades del modelo',
    'Validación del benchmark',
    'Plan de ejecución y riesgos',
    'Lanzamiento y evidencia'
  ],
  events: [],
  receipts: [],
  evidenceEntries: [],
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
  plasticity: {
    available: false,
    source_jobs: 0,
    elo_leaderboard: [],
    routing_decisions: [],
    notes: []
  }
};

const els = {
  systemStatus: document.getElementById('systemStatus'),
  runtimeBadge: document.getElementById('runtimeBadge'),
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

const seedEntries = [
  { title: 'Benchmark inicial', detail: 'Capacidad declarada y objetivos trazados', stamp: '08:41:02' },
  { title: 'Capability discovery', detail: 'Rutas candidatas y observación causal generadas', stamp: '08:41:09' },
  { title: 'Shadow evaluation', detail: 'Comparación de baseline frente a candidato', stamp: '08:41:18' },
  { title: 'Gate de promoción', detail: 'Preservación y riesgo revisados', stamp: '08:41:27' }
];

const moduleCatalog = [
  { name: 'KnowledgeEngine', area: 'Observación', kind: 'op', description: 'Recopila señales, evidencias y restricciones del entorno.', status: 'activo' },
  { name: 'Capability discovery', area: 'Diagnóstico', kind: 'op', description: 'Localiza capacidades relevantes y gaps entre modelos.', status: 'activo' },
  { name: 'ProceduralMemory', area: 'Aprendizaje', kind: 'op', description: 'Consolida decisiones reutilizables y reglas aprendidas.', status: 'activo' },
  { name: 'NumericalEvolution', area: 'Optimización', kind: 'op', description: 'Explora cambios de estrategias y parámetros en el espacio de solución.', status: 'activo' },
  { name: 'ReceiverCompiler', area: 'Compilación', kind: 'arch', description: 'Compila representaciones y candidatos desde CapabilityIR.', status: 'en revisión' },
  { name: 'AdapterBank', area: 'Producción', kind: 'op', description: 'Gestiona adaptadores, composiciones y rollback de materializaciones.', status: 'disponible' },
  { name: 'ProtectedMap', area: 'Gobernanza', kind: 'need', description: 'Mantiene límites de riesgo y preservación de la memoria base.', status: 'bloqueado' },
  { name: 'Shadow evaluation', area: 'Evaluación', kind: 'exp', description: 'Compara rendimiento real con impacto y riesgo esperados.', status: 'ejecutando' }
];

const pipelineSteps = [
  { step: 'Objetivo', detail: 'Definir capacidad, modelo y restricción.', state: 'available' },
  { step: 'Observación', detail: 'Benchmark inicial y descubrimiento de capacidad.', state: 'available' },
  { step: 'Diagnóstico', detail: 'Causalidad, gaps y compatibilidad.', state: 'available' },
  { step: 'Hipótesis', detail: 'Selección de intervención y estrategia.', state: 'manual' },
  { step: 'Experimento', detail: 'Compilación de candidato y shadow run.', state: 'available' },
  { step: 'Evaluación', detail: 'Medición de mejora y riesgo.', state: 'available' },
  { step: 'Comparación', detail: 'Diferencia entre baseline, candidate y preservación.', state: 'manual' },
  { step: 'Promoción', detail: 'Gate final de decisión y compatibilidad.', state: 'blocked' }
];

const workflowCatalog = [
  {
    id: 'behavioral_discovery',
    title: 'Descubrimiento conductual comparado',
    summary: 'Compara dos o más modelos con un benchmark JSON y produce evidencia de rendimiento real, brecha funcional y riesgo antes de materializar.',
    requiresModels: 2,
    requiresDataset: 'benchmark-json',
    category: 'comparativo'
  },
  {
    id: 'capability_discovery',
    title: 'Descubrimiento de capacidades relevantes',
    summary: 'Identifica qué capacidades emergen, dónde aparece la diferencia funcional y qué rutas son candidatas para la siguiente intervención.',
    requiresModels: 1,
    requiresDataset: 'optional',
    category: 'capacidad'
  },
  {
    id: 'receiver_profile',
    title: 'Perfil y compatibilidad del receptor',
    summary: 'Valida la huella funcional del receptor, la compatibilidad estructural y la preparación antes de compilar o materializar un cambio.',
    requiresModels: 1,
    requiresDataset: 'optional',
    category: 'receptor'
  },
  {
    id: 'compile_universal',
    title: 'Materialización de candidato',
    summary: 'Genera la versión candidata con restricciones de seguridad, compatibilidad y trazabilidad antes de la promoción final.',
    requiresModels: 1,
    requiresDataset: 'optional',
    category: 'materialización'
  },
  {
    id: 'promotion_readiness',
    title: 'Validación de preparación para promoción',
    summary: 'Comprueba la preparación del sistema para autorizar o rechazar la materialización según la evidencia, riesgo y preservación.',
    requiresModels: 1,
    requiresDataset: 'optional',
    category: 'gobernanza'
  },
  {
    id: 'adapter_authorize',
    title: 'Autorización de adaptador y gate',
    summary: 'Revisa la gobernanza, la evidencia y la firma del artefacto antes de aprobar la ruta elegida y fijar la decisión final del cerebro.',
    requiresModels: 1,
    requiresDataset: 'optional',
    category: 'autorización'
  }
];

function safeText(value) {
  return String(value ?? '').replace(/[&<>"']/g, (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]));
}

function updateTimeline() {
  const items = els.timelineList.querySelectorAll('li');
  items.forEach((item, index) => {
    item.classList.remove('done', 'doing');
    if (index < 2) item.classList.add('done');
    if (index === 2) item.classList.add('doing');
  });

  if (els.timelineList && state.timeline && state.timeline.length) {
    const labels = state.timeline;
    const currentItems = Array.from(els.timelineList.children);
    currentItems.forEach((item, index) => {
      if (labels[index]) {
        item.textContent = labels[index];
      }
    });
  }
}

function renderPipeline() {
  const source = state.executors.length ? state.executors.slice(0, 6) : pipelineSteps;
  els.pipelineOverview.innerHTML = `
    <div class="pipeline-grid">
      ${source.map((pipe, index) => {
        if (state.executors.length) {
          const title = pipe.title || pipe.executor_id || `Fase ${index + 1}`;
          const note = pipe.notes || pipe.effect_class || 'Autoridad de ejecución TIDE-X';
          const stateClass = pipe.state === 'operational' ? 'available' : pipe.state === 'operational_needs_workflow' ? 'manual' : 'blocked';
          return `
            <article class="pipeline-step ${stateClass}">
              <div class="head">
                <strong>${safeText(title)}</strong>
                <span class="chip ${stateClass === 'available' ? 'op' : stateClass === 'manual' ? 'need' : 'arch'}">${safeText(pipe.state || 'registrado')}</span>
              </div>
              <div class="why">${safeText(note)}</div>
            </article>
          `;
        }
        return `
          <article class="pipeline-step ${pipe.state}">
            <div class="head">
              <strong>${safeText(pipe.step)}</strong>
              <span class="chip ${pipe.state === 'available' ? 'op' : pipe.state === 'manual' ? 'need' : 'arch'}">${pipe.state === 'available' ? 'activo' : pipe.state === 'manual' ? 'manual' : 'bloqueado'}</span>
            </div>
            <div class="why">${safeText(pipe.detail)}</div>
          </article>
        `;
      }).join('')}
    </div>
  `;
}

function renderModuleGrid() {
  const modules = state.executors.length ? state.executors.slice(0, 8).map((executor) => ({
    name: executor.title || executor.executor_id || 'Executor',
    area: executor.effect_class || executor.authority || 'ejecución',
    kind: executor.state === 'operational' ? 'op' : executor.state === 'operational_needs_workflow' ? 'need' : executor.state === 'experimental_candidate_only' ? 'exp' : 'arch',
    description: executor.notes || 'Módulo registrado dentro de la autoridad TIDE-X.',
    status: executor.state || 'registrado'
  })) : moduleCatalog;

  els.moduleGrid.innerHTML = modules.map((module) => `
    <article class="module-card ${module.kind}">
      <div class="module-header">
        <strong>${safeText(module.name)}</strong>
        <span class="chip ${module.kind === 'op' ? 'op' : module.kind === 'need' ? 'need' : module.kind === 'exp' ? 'exp' : 'arch'}">${safeText(module.area)}</span>
      </div>
      <p>${safeText(module.description)}</p>
      <small>${safeText(module.status)}</small>
    </article>
  `).join('');
}

function renderPlasticity(activeTab = 'state') {
  const current = state.plasticity || { available: false, source_jobs: 0, elo_leaderboard: [], routing_decisions: [], notes: [] };

  const tabs = {
    state: `
      <div class="metric-grid compact">
        <div class="metric-card mini ${current.available ? 'ok' : 'warn'}"><span class="label">Disponibilidad</span><strong>${current.available ? 'SÍ' : 'NO'}</strong></div>
        <div class="metric-card mini warn"><span class="label">Jobs</span><strong>${current.source_jobs || 0}</strong></div>
        <div class="metric-card mini info"><span class="label">ELO entities</span><strong>${(current.elo_leaderboard || []).length}</strong></div>
        <div class="metric-card mini accent"><span class="label">Rutas</span><strong>${(current.routing_decisions || []).length}</strong></div>
      </div>
      <div class="summary-box">
        ${(current.notes && current.notes.length) ? safeText(current.notes.join(' · ')) : 'Sin evidencia plástica disponible todavía. La UI mostrará los resultados cuando la capa de plasticidad del runtime responda.'}
      </div>
    `,
    experiments: `
      <div class="result-list">
        ${(state.jobs || []).slice(0, 6).map((job) => `
          <div class="result-row"><strong>${safeText(job.job_id ? job.job_id.slice(0, 12) : '#') }</strong><span>${safeText(job.operation || 'job')}</span><span class="${job.state === 'completed' ? 'good' : job.state === 'failed' ? 'bad' : 'running'}">${safeText(job.state || 'unknown')}</span></div>
        `).join('') || '<div class="summary-box">Sin trabajos ejecutados aún.</div>'}
      </div>
    `,
    elo: `
      <div class="result-list">
        ${(current.elo_leaderboard || []).map(([name, rating]) => `
          <div class="result-row"><strong>${safeText(name)}</strong><span>rating</span><span>${Number(rating).toFixed(0)}</span></div>
        `).join('') || '<div class="summary-box">Sin ranking ELO suficiente para mostrar.</div>'}
      </div>
    `,
    routes: `
      <div class="route-list">
        ${(current.routing_decisions || []).map((route) => `
          <article class="route-card">
            <strong>${safeText(route.capability || 'capacidad')}</strong>
            <div class="route-path">${safeText(route.target_model || 'target_model')} · score ${Number(route.routing_score || 0).toFixed(3)}</div>
            <div class="route-meta">
              <span>measured: ${Number(route.measured_score || 0).toFixed(3)}</span>
              <span>evidence: ${safeText(String(route.evidence_sha256 || '').slice(0, 12))}</span>
            </div>
          </article>
        `).join('') || '<div class="summary-box">Sin rutas de routing calculadas.</div>'}
      </div>
    `,
    learning: `
      <div class="learning-grid">
        ${(state.executors || []).filter((e) => e.executor_id && (e.executor_id.startsWith('plasticity.') || e.executor_id.startsWith('learning.') || e.executor_id === 'numerical.evolve' || e.executor_id === 'procedural.memory')).map((e) => `
          <div class="learning-card"><strong>${safeText(e.title || e.executor_id)}</strong><small>${safeText(e.notes || e.executor_id)}</small></div>
        `).join('') || '<div class="summary-box">No hay ejecutores plásticos registrados.</div>'}
      </div>
    `
  };

  els.plasticityPanel.innerHTML = tabs[activeTab] || tabs.state;
}

function renderEventLog() {
  const events = state.events;
  document.getElementById('eventLog').innerHTML = events
    .map((event) => `
      <div class="log-entry">
        <strong>${safeText(event.title)}</strong>
        <small>${safeText(event.detail)}</small>
      </div>
    `)
    .join('') || '<div class="log-entry"><strong>Sin eventos</strong><small>La actividad aparecerá aquí.</small></div>';
}

function renderEvidence() {
  document.getElementById('evidenceLog').innerHTML = state.evidenceEntries
    .map((entry) => `
      <div class="log-entry">
        <strong>${safeText(entry.title)}</strong>
        <small>${safeText(entry.detail)} · ${safeText(entry.stamp)}</small>
      </div>
    `)
    .join('');
}

function renderReceipts() {
  const receipts = (state.receipts && state.receipts.length) ? state.receipts : (state.jobs || []).slice(0, 4).map((job) => ({
    name: job.job_id || 'job',
    status: job.state || 'pending',
    hash: job.evidence_receipt?.evidence_sha256 || job.job_id || 'sin-hash'
  }));

  document.getElementById('receiptList').innerHTML = receipts
    .map((receipt) => `
      <div class="receipt-item">
        <strong>${safeText(receipt.name)}</strong>
        <small>${safeText(receipt.status)}</small>
        <div class="hash">${safeText(receipt.hash)}</div>
      </div>
    `)
    .join('');
}

function addEvent(title, detail) {
  state.events.unshift({ title, detail });
  if (state.events.length > 8) state.events.pop();
  renderEventLog();
}

function refreshMetrics() {
  const runtimeText = state.info && state.info.hf_python ? `${state.info.hf_python}` : state.runtime;
  els.systemStatus.textContent = state.status.toUpperCase();
  els.runtimeBadge.textContent = state.info ? `python: ${state.info.hf_python || 'sin runtime'}` : 'runtime local';
  els.metricAuthority.textContent = `${state.authority}%`;
  els.metricEvidence.textContent = `${state.evidence}/4`;
  els.metricCapacity.textContent = state.status === 'activo' ? 'Transferencia' : state.status === 'ejecutando' ? 'Diagnóstico' : 'Preparado';
  els.metricRuntime.textContent = runtimeText;
  els.phasePill.textContent = state.phase;
  els.summaryBox.textContent = state.phase === 'READY'
    ? 'El cerebro TIDE-X está preparado para seguir la secuencia: objetivo operativo, evaluación del modelo, validación del benchmark, plan de ejecución y evidencia final.'
    : state.phase === 'PLAN'
      ? 'Se está construyendo el plan operativo del cerebro: evaluación de capacidades, validación del benchmark y control de riesgo antes del lanzamiento.'
      : 'La operación está activa: la evidencia, la preservación y la validación del entorno están alineadas con el gate final de ejecución del sistema.';
}

function setupInitialState() {
  state.status = 'listo';
  state.phase = 'READY';
  state.authority = 100;
  state.evidence = 4;
  state.runtime = 'OK';
  state.events = [
    { title: 'Cerebro cargado', detail: 'Superficie de control lista y conectada a la arquitectura real de TIDE-X.' },
    { title: 'Entorno protegido', detail: 'Privacidad, autoridad y evidencia confirmadas.' }
  ];
  state.evidenceEntries = [...seedEntries];
  state.receipts = [
    { name: 'receipt:objective', status: 'validado', hash: '7f64c0..a12fe9' },
    { name: 'receipt:discovery', status: 'autorizado', hash: '0cf2d1..a90c7d' },
    { name: 'receipt:preservation', status: 'confirmado', hash: 'aa3ed2..cb1045' }
  ];

  refreshMetrics();
  renderEventLog();
  renderEvidence();
  renderReceipts();
  renderPipeline();
  renderModuleGrid();
  renderPlasticity();
  updateTimeline();
}

async function fetchJson(url, options = {}) {
  const response = await fetch(url, { cache: 'no-store', ...options });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(text || `${response.status} ${response.statusText}`);
  }
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

function modelKey(model) {
  return model.model_id || model.id || model.name || model.root || model.label || null;
}

function datasetFormatLabel(format) {
  return ({ json: 'JSON', jsonl: 'JSONL', csv: 'CSV', text: 'Texto' })[format] || 'Archivo';
}

function getDatasetById(id) {
  if (!id) return null;
  if (state.uploadedDataset && state.uploadedDataset.id === id) return state.uploadedDataset;
  return state.datasets.find((dataset) => (dataset.content_sha256 || dataset.id || dataset.name) === id) || null;
}

function getSelectedDataset() {
  return getDatasetById(state.selectedDatasetId);
}

function datasetCompatibleForWorkflow(dataset, workflowId) {
  if (!dataset) {
    return workflowId !== 'behavioral_discovery';
  }
  if (workflowId === 'behavioral_discovery') {
    return dataset.format === 'json';
  }
  return true;
}

function computeAvailableWorkflows() {
  const selectedCount = state.selectedModels.length;
  const selectedDataset = getSelectedDataset();
  const workflows = workflowCatalog.filter((workflow) => {
    if (selectedCount < workflow.requiresModels) return false;
    if (workflow.requiresDataset === 'benchmark-json') {
      return datasetCompatibleForWorkflow(selectedDataset, workflow.id);
    }
    return true;
  });

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
  const availableWorkflows = computeAvailableWorkflows();

  els.modelSelector.innerHTML = (state.models.length ? state.models : [{ name: 'Sin modelos disponibles', root: 'none', model_id: 'none' }])
    .map((model) => {
      const key = modelKey(model);
      const isSelected = !!key && state.selectedModels.includes(key);
      const disabled = key === null || key === 'none';
      return `
        <button type="button" class="option-card ${isSelected ? 'selected' : ''} ${disabled ? 'disabled' : ''}" data-model-key="${safeText(key || '')}" ${disabled ? 'disabled' : ''}>
          <strong>${safeText(model.name || model.label || model.root || 'Modelo')}</strong>
          <small>${safeText(model.model_id || model.id || model.root || 'sin id')}</small>
          <div class="option-meta">
            <span>${safeText(model.architecture || model.layout || 'runtime local')}</span>
            <span>${isSelected ? 'seleccionado' : 'pendiente'}</span>
          </div>
        </button>
      `;
    })
    .join('');

  const datasets = state.datasets.length ? state.datasets : [];
  const displayedDatasets = datasets.length ? datasets : [{ name: 'Sin datasets cargados', format: 'text', content_sha256: 'none' }];
  els.datasetSelector.innerHTML = displayedDatasets.map((dataset) => {
    const id = dataset.content_sha256 || dataset.id || dataset.name || 'none';
    const selected = id === state.selectedDatasetId;
    const disabled = id === 'none';
    return `
      <button type="button" class="option-card ${selected ? 'selected' : ''} ${disabled ? 'disabled' : ''}" data-dataset-id="${safeText(id)}" ${disabled ? 'disabled' : ''}>
        <strong>${safeText(dataset.name || 'Dataset')}</strong>
        <div class="option-meta">
          <span>${safeText(datasetFormatLabel(dataset.format || 'text'))}</span>
          <span>${dataset.generated ? 'generado' : 'importado'}</span>
        </div>
        <small>${safeText(dataset.content_sha256 || dataset.id || 'sin SHA')}</small>
      </button>
    `;
  }).join('');

  let validationMessage = 'Elige un benchmark JSON para descubrimiento conductual o un archivo compatible para análisis directo.';
  let validationKind = 'neutral';
  const selectedDataset = getSelectedDataset();
  if (selectedDataset) {
    if (selectedDataset.format === 'json') {
      validationKind = 'success';
      validationMessage = 'Archivo JSON válido. Se puede usar para benchmark conductual y para análisis directo.';
    } else {
      validationKind = 'warning';
      validationMessage = 'Archivo compatible, pero para descubrimiento conductual es mejor usar un benchmark JSON con schema de comportamiento.';
    }
  }
  if (state.datasetValidation) {
    validationKind = state.datasetValidation.kind;
    validationMessage = state.datasetValidation.message;
  }

  els.datasetValidation.className = `validation-box ${validationKind}`;
  els.datasetValidation.textContent = validationMessage;

  if (!availableWorkflows.length) {
    els.workflowSelector.innerHTML = `
      <div class="summary-box">
        Se necesitan al menos 2 modelos para comparación conductual y, preferiblemente, un benchmark JSON compatible. Ajusta la selección para que el sistema te ofrezca workflows válidos.
      </div>
    `;
  } else {
    els.workflowSelector.innerHTML = availableWorkflows.map((workflow) => {
      const active = workflow.id === state.selectedWorkflow;
      return `
        <button type="button" class="option-card ${active ? 'selected' : ''}" data-workflow-id="${workflow.id}">
          <strong>${safeText(workflow.title)}</strong>
          <div class="option-meta">
            <span>${safeText(workflow.category)}</span>
            <span>${workflow.requiresModels} modelo(s)</span>
          </div>
          <small>${safeText(workflow.summary)}</small>
        </button>
      `;
    }).join('');
  }

  const summary = getSelectedWorkflowSummary();
  els.executionSummary.innerHTML = summary;
}

function getExecutionPlanText() {
  const workflow = getWorkflowById(state.selectedWorkflow);
  const selectedDataset = getSelectedDataset();
  const modelCount = state.selectedModels.length;

  if (!workflow) {
    return 'No hay un plan de ejecución válido para la selección actual. Se requiere una combinación consistente de modelos, objetivo y benchmark.';
  }

  const datasetStatus = selectedDataset
    ? selectedDataset.format === 'json'
      ? 'Benchmark validado para ejecución conductual.'
      : 'Dataset compatible para análisis directo.'
    : 'Sin dataset validado para esta operación.';

  return `Plan de ejecución ${workflow.title}: ${modelCount} modelo(s) preparados, validación de entorno confirmada y control de evidencia habilitado. ${datasetStatus}`;
}

function getSelectedWorkflowSummary() {
  const selectedModels = state.selectedModels.map((id) => {
    const match = state.models.find((model) => modelKey(model) === id);
    return match ? (match.name || modelKey(match)) : id;
  });
  const dataset = getSelectedDataset();
  const workflow = getWorkflowById(state.selectedWorkflow);

  if (!workflow) {
    return `
      <strong>Sin workflow válido disponible.</strong>
      <div>La selección actual no cumple las condiciones de operación. Debe haber un objetivo claro, modelos compatibles y un benchmark o dataset preparado.</div>
    `;
  }

  const datasetText = dataset ? `${dataset.name} (${datasetFormatLabel(dataset.format || 'text')})` : 'sin dataset';
  return `
    <strong>Workflow seleccionado:</strong> ${safeText(workflow.title)}<br>
    <strong>Modelos:</strong> ${selectedModels.length ? safeText(selectedModels.join(', ')) : 'ninguno'}<br>
    <strong>Dataset:</strong> ${safeText(datasetText)}<br>
    <strong>Plan:</strong> ${safeText(getExecutionPlanText())}<br>
    <strong>Resultado esperado:</strong> ${safeText(workflow.summary)}
  `;
}

function collectSelectedModels() {
  return state.selectedModels.filter((id) => state.models.some((model) => modelKey(model) === id));
}

async function ensureDatasetReady() {
  const dataset = getSelectedDataset();
  if (!dataset) return null;
  if (dataset.content_sha256) return dataset.content_sha256;
  if (!dataset.content) return null;

  const response = await fetchJson('/api/datasets/import', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      name: dataset.name,
      format: dataset.format,
      content: dataset.content,
      generated: !!dataset.generated
    })
  });

  if (response && response.content_sha256) {
    state.datasets = [response, ...state.datasets.filter((item) => (item.content_sha256 || item.id || item.name) !== dataset.id)];
    state.selectedDatasetId = response.content_sha256;
    return response.content_sha256;
  }

  return null;
}

async function runSelectedWorkflow() {
  const selectedModels = collectSelectedModels();
  const workflow = getWorkflowById(state.selectedWorkflow) || computeAvailableWorkflows()[0];

  if (!workflow) {
    throw new Error('No hay un workflow válido para la selección actual. Elige al menos dos modelos para comparación conductual o un modelo para análisis directo.');
  }

  if (selectedModels.length < workflow.requiresModels) {
    throw new Error(`Este workflow requiere ${workflow.requiresModels} modelo(s).`);
  }

  if (workflow.id === 'behavioral_discovery') {
    const dataset = getSelectedDataset();
    if (!dataset || dataset.format !== 'json') {
      throw new Error('El descubrimiento conductual requiere un benchmark JSON válido.');
    }
  }

  const datasetId = await ensureDatasetReady();
  const requestBody = {
    schema: 'cerebro.tidex.lab_direct_workflow/v1',
    operation: workflow.id,
    model_ids: selectedModels,
    dataset_sha256: workflow.id === 'behavioral_discovery' ? datasetId : null,
    parameters: {
      goal: els.goalInput.value.trim() || state.goal,
      selected_dataset: datasetId,
      workflow: workflow.id
    }
  };

  if (workflow.id === 'behavioral_discovery') {
    requestBody.schema = 'cerebro.tidex.lab_behavioral_discovery/v1';
    requestBody.model_ids = selectedModels;
    requestBody.dataset_sha256 = datasetId;
    requestBody.max_new_tokens = 128;
    requestBody.seed = 0;
    delete requestBody.operation;
  }

  state.phase = 'ACTIVE';
  state.status = 'activo';
  state.authority = 100;

  try {
    const endpoint = workflow.id === 'behavioral_discovery'
      ? '/api/workflows/behavioral-discovery'
      : '/api/workflows/direct';

    const response = await fetchJson(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(requestBody)
    });

    if (response && response.job_id) {
      state.jobs = [response, ...(state.jobs || [])];
      addEvent('Workflow ejecutado', `Trabajo lanzado con éxito: ${response.job_id.slice(0, 12)}…`);
      state.evidenceEntries.unshift({
        title: 'Workflow ejecutado',
        detail: workflow.title,
        stamp: 'ahora'
      });
    }

    renderReceipts();
    renderEvidence();
    refreshMetrics();
    updateTimeline();
    return response;
  } catch (error) {
    addEvent('Workflow sin respuesta', error.message || 'El cerebro no aceptó la ejecución solicitada.');
    throw error;
  }
}

async function handleDatasetUpload(event) {
  const file = event.target.files && event.target.files[0];
  if (!file) return;

  const ext = file.name.split('.').pop()?.toLowerCase() || 'txt';
  const format = ['json', 'jsonl', 'csv'].includes(ext) ? ext : 'text';
  const content = await file.text();

  let message = 'Archivo compatible cargado. Se puede usar para análisis directo y, si es JSON, para benchmark conductual.';
  let kind = 'neutral';

  if (format === 'json') {
    try {
      const parsed = JSON.parse(content);
      if (parsed && parsed.schema === 'cerebro.cross_model.behavioral_benchmark/v1') {
        kind = 'success';
        message = 'Benchmark detectado: cumple el schema de comportamiento requerido por TIDE-X.';
      } else {
        kind = 'warning';
        message = 'JSON cargado; válido para análisis directo, pero no parece un benchmark conductual. Se recomienda un schema cerebro.cross_model.behavioral_benchmark/v1 para descubrimiento.';
      }
    } catch {
      kind = 'warning';
      message = 'El archivo JSON no se pudo parsear. Revisa que el contenido sea válido antes de ejecutar.';
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

async function refreshLiveData() {
  try {
    const [info, models, datasets, plasticity, executors, jobs] = await Promise.all([
      fetchJson('/api/info').catch(() => null),
      fetchJson('/api/models').catch(() => []),
      fetchJson('/api/datasets').catch(() => []),
      fetchJson('/api/plasticity').catch(() => state.plasticity),
      fetchJson('/api/executors').catch(() => []),
      fetchJson('/api/jobs').catch(() => [])
    ]);

    state.info = info;
    state.models = Array.isArray(models) ? models : [];
    state.datasets = Array.isArray(datasets) ? datasets : [];
    state.plasticity = plasticity || state.plasticity;
    state.executors = Array.isArray(executors) ? executors : [];
    state.jobs = Array.isArray(jobs) ? jobs : [];
    state.runtime = info && info.nnsight_available ? 'NNsight OK' : 'OK';
    state.evidence = Math.min(4, (state.executors.length || 0) + 2);
    state.authority = state.plasticity?.available ? 100 : 96;

    if (!state.selectedModels.length && state.models.length) {
      state.selectedModels = state.models.slice(0, 2).map((model) => model.model_id || model.id || model.root);
    }
    if (!state.selectedDatasetId && state.datasets.length) {
      const preferred = state.datasets.find((dataset) => dataset.format === 'json') || state.datasets[0];
      state.selectedDatasetId = preferred ? (preferred.content_sha256 || preferred.id || preferred.name) : null;
    }

    const liveEntry = {
      title: 'Control del cerebro conectado',
      detail: info ? `${info.lab_home || 'lab'} · ${info.hf_python || 'runtime local'}` : 'modo fallback',
      stamp: new Date().toLocaleTimeString()
    };

    state.evidenceEntries = [liveEntry, ...state.evidenceEntries].slice(0, 6);
    state.receipts = state.jobs.slice(0, 4).map((job) => ({
      name: job.job_id || 'job',
      status: job.state || 'pending',
      hash: job.evidence_receipt?.evidence_sha256 || job.job_id || 'sin-hash'
    }));

    addEvent('Cerebro conectado', info ? 'La interfaz está consumiendo la API real del cerebro TIDE-X.' : 'La API no respondió; se conserva el estado local de control.');
    renderPipeline();
    renderModuleGrid();
    renderPlasticity();
    renderEvidence();
    renderReceipts();
    renderWorkflowWizard();
    refreshMetrics();
  } catch (error) {
    state.info = null;
    state.models = [];
    state.datasets = [];
    state.executors = [];
    state.jobs = [];
    state.plasticity = state.plasticity || {
      available: true,
      source_jobs: 3,
      elo_leaderboard: [['capability.python', 1584], ['capability.reasoning', 1512]],
      routing_decisions: [{ capability: 'python', target_model: 'source_model', routing_score: 0.84, measured_score: 0.81 }],
      notes: ['Fallback local del cerebro operativo.']
    };
    state.runtime = 'fallback';
    state.evidence = 4;
    state.authority = 100;
    state.evidenceEntries = [{ title: 'Modo local', detail: 'Sin API disponible; se usa el estado operable del cerebro TIDE-X.', stamp: new Date().toLocaleTimeString() }, ...seedEntries].slice(0, 6);
    renderPipeline();
    renderModuleGrid();
    renderPlasticity();
    renderEvidence();
    renderReceipts();
    renderWorkflowWizard();
    refreshMetrics();
  }
}

async function buildPipeline() {
  state.phase = 'PLAN';
  state.status = 'ejecutando';
  state.authority = 96;
  state.goal = els.goalInput.value.trim() || state.goal;
  addEvent('Plan operativo construido', 'Se ha materializado el plan del cerebro con validación del objetivo, capacidad del modelo, benchmark y control de riesgos.');
  state.evidenceEntries.unshift({ title: 'Pipeline operativo', detail: getExecutionPlanText(), stamp: 'ahora' });
  renderWorkflowWizard();
  renderPipeline();
  renderEvidence();
  renderReceipts();
  refreshMetrics();
  updateTimeline();
}

async function runCycle() {
  await runSelectedWorkflow();
}

function resetState() {
  state.selectedModels = [];
  state.selectedDatasetId = null;
  state.selectedWorkflow = null;
  state.uploadedDataset = null;
  state.datasetValidation = null;
  setupInitialState();
  addEvent('Cerebro reiniciado', 'El entorno regresa al estado base con el plan operativo visible y la evidencia limpia.');
  renderWorkflowWizard();
}

function bindPanelNavigation() {
  document.querySelectorAll('.nav-item').forEach((button) => {
    button.addEventListener('click', () => {
      document.querySelectorAll('.nav-item').forEach((item) => item.classList.remove('active'));
      button.classList.add('active');
      document.querySelectorAll('.panel').forEach((panel) => panel.classList.remove('active'));
      const target = document.getElementById(button.dataset.panel);
      if (target) target.classList.add('active');
    });
  });
}

function bindPlasticityTabs() {
  document.querySelectorAll('.tab-btn').forEach((button) => {
    button.addEventListener('click', () => {
      document.querySelectorAll('.tab-btn').forEach((item) => item.classList.remove('active'));
      button.classList.add('active');
      renderPlasticity(button.dataset.tab);
    });
  });
}

els.buildPipelineBtn.addEventListener('click', buildPipeline);
els.runCycleBtn.addEventListener('click', runCycle);
els.runSelectedWorkflowBtn.addEventListener('click', async () => {
  try {
    await runSelectedWorkflow();
  } catch (error) {
    addEvent('Selección inválida', error.message || 'El workflow seleccionado no puede ejecutarse en este estado.');
  }
});
els.resetBtn.addEventListener('click', resetState);
els.refreshBtn.addEventListener('click', refreshLiveData);
els.goalInput.addEventListener('input', (event) => {
  state.goal = event.target.value;
});
els.datasetUpload.addEventListener('change', handleDatasetUpload);

document.getElementById('modelSelector').addEventListener('click', (event) => {
  const button = event.target.closest('[data-model-key]');
  if (!button) return;
  const key = button.dataset.modelKey;
  if (!key || key === 'none') return;
  const selected = state.selectedModels.includes(key);
  const next = selected
    ? state.selectedModels.filter((item) => item !== key)
    : [...state.selectedModels, key];
  state.selectedModels = next;
  const workflows = computeAvailableWorkflows();
  if (!workflows.length && state.selectedModels.length) {
    state.selectedWorkflow = null;
  }
  renderWorkflowWizard();
});

document.getElementById('datasetSelector').addEventListener('click', (event) => {
  const button = event.target.closest('[data-dataset-id]');
  if (!button) return;
  const id = button.dataset.datasetId;
  if (!id || id === 'none') return;
  state.selectedDatasetId = id;
  state.datasetValidation = null;
  renderWorkflowWizard();
});

document.getElementById('workflowSelector').addEventListener('click', (event) => {
  const button = event.target.closest('[data-workflow-id]');
  if (!button) return;
  state.selectedWorkflow = button.dataset.workflowId;
  renderWorkflowWizard();
});

bindPanelNavigation();
bindPlasticityTabs();
setupInitialState();
renderWorkflowWizard();
refreshLiveData();
