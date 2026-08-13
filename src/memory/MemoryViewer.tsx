import { type FormEvent, useCallback, useEffect, useState } from "react";
import {
  archiveMemory,
  consolidateMemories,
  deleteMemory,
  editMemory,
  getMemoryDetail,
  getMemoryViewer,
  pinMemory,
  restoreMemory,
} from "./manager";
import type {
  Memory,
  MemoryDetail,
  MemoryKind,
  MemoryStatus,
  MemoryViewerSnapshot,
} from "./types";

type FilterValue<T extends string> = T | "all";

type EditDraft = {
  content: string;
  kind: MemoryKind;
  importance: number;
  confidence: number;
};

export function MemoryViewer({
  onError,
  onChanged,
  previewMode = false,
}: {
  onError: (message?: string) => void;
  onChanged: () => void;
  previewMode?: boolean;
}) {
  const [kind, setKind] = useState<FilterValue<MemoryKind>>("all");
  const [status, setStatus] = useState<FilterValue<MemoryStatus>>("all");
  const [query, setQuery] = useState("");
  const [activeQuery, setActiveQuery] = useState("");
  const [snapshot, setSnapshot] = useState<MemoryViewerSnapshot>();
  const [detail, setDetail] = useState<MemoryDetail>();
  const [editDraft, setEditDraft] = useState<EditDraft>();
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    if (previewMode) {
      setSnapshot(PREVIEW_SNAPSHOT);
      return;
    }
    try {
      const next = await getMemoryViewer({
        kind: kind === "all" ? undefined : kind,
        status: status === "all" ? undefined : status,
        query: activeQuery || undefined,
      });
      setSnapshot(next);
      if (detail && !next.memories.some(({ id }) => id === detail.memory.id)) {
        setDetail(undefined);
        setEditDraft(undefined);
      }
    } catch (error) {
      onError(readError(error));
    }
  }, [activeQuery, detail, kind, onError, previewMode, status]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function selectMemory(memory: Memory) {
    onError(undefined);
    try {
      setDetail(await getMemoryDetail(memory.id));
      setEditDraft(undefined);
    } catch (error) {
      onError(readError(error));
    }
  }

  async function afterMutation(memoryId?: string) {
    await refresh();
    if (memoryId) setDetail(await getMemoryDetail(memoryId));
    onChanged();
  }

  async function runMutation(
    action: () => Promise<unknown>,
    memoryId?: string,
  ) {
    setBusy(true);
    onError(undefined);
    try {
      await action();
      await afterMutation(memoryId);
    } catch (error) {
      onError(readError(error));
    } finally {
      setBusy(false);
    }
  }

  function submitSearch(event: FormEvent) {
    event.preventDefault();
    setActiveQuery(query.trim());
  }

  function beginEdit(memory: Memory) {
    setEditDraft({
      content: memory.content,
      kind: memory.kind,
      importance: memory.importance,
      confidence: memory.confidence,
    });
  }

  async function saveEdit() {
    if (!detail || !editDraft) return;
    await runMutation(
      () => editMemory(detail.memory.id, editDraft),
      detail.memory.id,
    );
    setEditDraft(undefined);
  }

  async function trulyDelete(memory: Memory) {
    if (
      !window.confirm(
        "Delete this long-term memory permanently? This cannot be undone.",
      )
    ) {
      return;
    }
    await runMutation(() => deleteMemory(memory.id));
    setDetail(undefined);
    setEditDraft(undefined);
  }

  const counts = snapshot?.counts;

  return (
    <section className="settings-section memory-inspector">
      <div className="section-heading">
        <div>
          <h2>Memory</h2>
          <p>Inspect and correct what Remi remembers. No LLM is used here.</p>
        </div>
        <button
          type="button"
          disabled={busy}
          onClick={() =>
            void runMutation(() => consolidateMemories()).then(() =>
              setDetail(undefined),
            )
          }
        >
          Consolidate
        </button>
      </div>

      <div className="memory-counts" aria-label="Memory status counts">
        <Count label="Active" value={counts?.active} />
        <Count label="Outdated" value={counts?.outdated} />
        <Count label="Archived" value={counts?.archived} />
        <Count label="Total" value={counts?.total} />
      </div>

      <form className="memory-search" onSubmit={submitSearch}>
        <input
          type="search"
          value={query}
          placeholder="Search memories…"
          onChange={(event) => setQuery(event.target.value)}
        />
        <button type="submit">Search</button>
        {activeQuery && (
          <button
            type="button"
            className="secondary-button"
            onClick={() => {
              setQuery("");
              setActiveQuery("");
            }}
          >
            Clear
          </button>
        )}
      </form>

      <div className="memory-filters">
        <label>
          Type
          <select
            value={kind}
            onChange={(event) =>
              setKind(event.target.value as FilterValue<MemoryKind>)
            }
          >
            <option value="all">All</option>
            <option value="semantic">Semantic</option>
            <option value="episodic">Episodic</option>
            <option value="relationship">Relationship</option>
          </select>
        </label>
        <label>
          Status
          <select
            value={status}
            onChange={(event) =>
              setStatus(event.target.value as FilterValue<MemoryStatus>)
            }
          >
            <option value="all">All</option>
            <option value="active">Active</option>
            <option value="outdated">Outdated</option>
            <option value="archived">Archived</option>
            <option value="merged">Merged</option>
          </select>
        </label>
      </div>

      <div className="memory-inspector-grid">
        <div className="memory-list" aria-label="Long-term memories">
          {(snapshot?.memories ?? []).map((memory) => (
            <button
              type="button"
              className={`memory-row ${
                detail?.memory.id === memory.id ? "selected" : ""
              }`}
              key={memory.id}
              onClick={() => void selectMemory(memory)}
            >
              <header>
                <span>{memory.kind}</span>
                <span className={`memory-status ${memory.status}`}>
                  {memory.status}
                </span>
                {memory.pinned && <span className="memory-pin">Pinned</span>}
              </header>
              <p>{memory.content}</p>
              <footer>
                Source: {memory.sourceType ?? "unknown"} · importance{" "}
                {memory.importance.toFixed(2)} · confidence{" "}
                {memory.confidence.toFixed(2)}
              </footer>
              <small>
                Created {formatDate(memory.createdAt)} · Updated{" "}
                {formatDate(memory.updatedAt)}
              </small>
            </button>
          ))}
          {snapshot?.memories.length === 0 && (
            <p className="settings-note">No memories match this filter.</p>
          )}
        </div>

        <div className="memory-detail" aria-live="polite">
          {!detail && (
            <p className="settings-note">
              Select a memory to view provenance, relations, and controls.
            </p>
          )}
          {detail && (
            <>
              <div className="memory-detail-heading">
                <strong>Memory Detail</strong>
                <span className={`memory-status ${detail.memory.status}`}>
                  {detail.memory.status}
                </span>
              </div>

              {editDraft ? (
                <div className="memory-edit-form">
                  <label>
                    Content
                    <textarea
                      value={editDraft.content}
                      onChange={(event) =>
                        setEditDraft({
                          ...editDraft,
                          content: event.target.value,
                        })
                      }
                    />
                  </label>
                  <label>
                    Type
                    <select
                      value={editDraft.kind}
                      onChange={(event) =>
                        setEditDraft({
                          ...editDraft,
                          kind: event.target.value as MemoryKind,
                        })
                      }
                    >
                      <option value="semantic">Semantic</option>
                      <option value="episodic">Episodic</option>
                      <option value="relationship">Relationship</option>
                    </select>
                  </label>
                  <div className="memory-score-grid">
                    <label>
                      Importance
                      <input
                        type="number"
                        min="0"
                        max="1"
                        step="0.05"
                        value={editDraft.importance}
                        onChange={(event) =>
                          setEditDraft({
                            ...editDraft,
                            importance: Number(event.target.value),
                          })
                        }
                      />
                    </label>
                    <label>
                      Confidence
                      <input
                        type="number"
                        min="0"
                        max="1"
                        step="0.05"
                        value={editDraft.confidence}
                        onChange={(event) =>
                          setEditDraft({
                            ...editDraft,
                            confidence: Number(event.target.value),
                          })
                        }
                      />
                    </label>
                  </div>
                  <div className="memory-actions">
                    <button
                      type="button"
                      disabled={busy || !editDraft.content.trim()}
                      onClick={() => void saveEdit()}
                    >
                      Save Correction
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => setEditDraft(undefined)}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
                <p className="memory-detail-content">{detail.memory.content}</p>
              )}

              <dl className="memory-metadata">
                <dt>Type</dt>
                <dd>{detail.memory.kind}</dd>
                <dt>Source</dt>
                <dd>{detail.memory.sourceType ?? "unknown"}</dd>
                <dt>Source event</dt>
                <dd>{detail.memory.sourceRef ?? "—"}</dd>
                <dt>Importance</dt>
                <dd>{detail.memory.importance.toFixed(2)}</dd>
                <dt>Confidence</dt>
                <dd>{detail.memory.confidence.toFixed(2)}</dd>
                <dt>Created</dt>
                <dd>{formatDate(detail.memory.createdAt)}</dd>
                <dt>Updated</dt>
                <dd>{formatDate(detail.memory.updatedAt)}</dd>
                <dt>Valid from</dt>
                <dd>{formatOptionalDate(detail.memory.validFrom)}</dd>
                <dt>Valid to</dt>
                <dd>{formatOptionalDate(detail.memory.validTo)}</dd>
              </dl>

              <div className="memory-relations">
                <strong>Relations</strong>
                {detail.relations.map((relation) => (
                  <span
                    key={`${relation.sourceId}-${relation.targetId}-${relation.relation}`}
                  >
                    {shortId(relation.sourceId)}{" "}
                    {relation.relation.replace("_", " ")} →{" "}
                    {shortId(relation.targetId)}
                  </span>
                ))}
                {detail.relations.length === 0 && <span>None</span>}
              </div>

              <div className="memory-actions">
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => beginEdit(detail.memory)}
                >
                  Edit
                </button>
                {detail.memory.status === "active" && (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() =>
                      void runMutation(
                        () => archiveMemory(detail.memory.id),
                        detail.memory.id,
                      )
                    }
                  >
                    Archive
                  </button>
                )}
                {detail.memory.status === "archived" && (
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() =>
                      void runMutation(
                        () => restoreMemory(detail.memory.id),
                        detail.memory.id,
                      )
                    }
                  >
                    Restore
                  </button>
                )}
                <button
                  type="button"
                  disabled={busy || detail.memory.pinned}
                  title={detail.memory.pinned ? "Already pinned" : undefined}
                  onClick={() =>
                    void runMutation(
                      () => pinMemory(detail.memory.id),
                      detail.memory.id,
                    )
                  }
                >
                  {detail.memory.pinned ? "Pinned" : "Pin"}
                </button>
                <button
                  type="button"
                  className="danger-text"
                  disabled={busy}
                  onClick={() => void trulyDelete(detail.memory)}
                >
                  Delete
                </button>
              </div>
            </>
          )}
        </div>
      </div>

      <div className="memory-trace">
        <h3>Recent Memory Operations</h3>
        {(snapshot?.operations ?? []).slice(0, 40).map((operation) => (
          <div key={operation.id}>
            <time>{formatTime(operation.timestamp)}</time>
            <strong>{operation.operation}</strong>
            <span>{operation.reasonLabel ?? "unspecified"}</span>
            <code>
              {operation.memoryId ? shortId(operation.memoryId) : "—"}
            </code>
          </div>
        ))}
      </div>
    </section>
  );
}

const PREVIEW_SNAPSHOT: MemoryViewerSnapshot = {
  memories: [],
  relations: [],
  operations: [],
  counts: { active: 0, outdated: 0, archived: 0, merged: 0, total: 0 },
  policyVersion: "evolving-memory-v1",
};

function Count({ label, value }: { label: string; value?: number }) {
  return (
    <div>
      <span>{label}</span>
      <strong>{value ?? "…"}</strong>
    </div>
  );
}

function readError(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function shortId(value: string) {
  return value.slice(0, 8);
}

function formatDate(value: number) {
  return new Date(value).toLocaleString();
}

function formatOptionalDate(value?: number) {
  return value ? formatDate(value) : "—";
}

function formatTime(value: number) {
  return new Date(value).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
}
