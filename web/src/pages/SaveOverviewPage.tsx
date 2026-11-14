import { reasonText } from '../i18n/reasons';
import { Feedback, type FeedbackMessage } from '../i18n/Feedback';
import { describeError } from '../i18n/errors';
import { t } from '../i18n/index';
import { Settings } from '../i18n/Settings';
import { useCallback, useEffect, useRef, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  applyHeroEdit,
  applyPlayerEdit,
  applyTentEdit,
  applyTeamEdit,
  listPlayers,
  discardPlayerEdits,
  applyTownEdit,
  chooseSave,
  discardAllEdits,
  discardHeroEdits,
  setHeroDismissal,
  discardTownEdits,
  getAppInfo,
  getHero,
  getHeroEditState,
  getPendingEdits,
  getTown,
  getTownEditState,
  listHeroes,
  listTowns,
  openGrantedSave,
  saveSaveAs,
  save,
} from '../api/save';
import {
  chooseInstallation,
  getHeroPortraits,
  getInstallationStatus,
} from '../api/resources';
import { PlayersPanel } from '../components/PlayersPanel';
import { CreaturesPanel } from '../components/CreaturesPanel';
import { HeroDetail } from '../components/HeroDetail';
import { HeroList } from '../components/HeroList';
import { SaveBanner } from '../components/SaveBanner';
import { TownDetail } from '../components/TownDetail';
import { TownList } from '../components/TownList';
import { decodeText, type LegacyEncoding } from '../lib/legacyText';
import type {
  AppInfo,
  PlayerDto,
  HeroDetailDto,
  HeroEditRequest,
  HeroEditResponseDto,
  HeroListItemDto,
  InstallationDto,
  PendingEditDto,
  PortraitDto,
  SaveSummaryDto,
  TownDetailDto,
  TownEditRequest,
  TownEditResponseDto,
  TownListItemDto,
} from '../types/api';

type OpenState =
  | { phase: 'idle' }
  | { phase: 'opening' }
  | { phase: 'ready'; summary: SaveSummaryDto; heroes: HeroListItemDto[]; towns: TownListItemDto[] }
  | { phase: 'error'; message: FeedbackMessage };

const errorMessage = describeError;

const ENCODING_STORAGE_KEY = 'h3forge.encoding';

function initialEncoding(): LegacyEncoding {
  try {
    const saved = localStorage.getItem(ENCODING_STORAGE_KEY);
    return saved === 'cp1251' ? 'cp1251' : 'auto';
  } catch {
    return 'auto';
  }
}

export function SaveOverviewPage() {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [openState, setOpenState] = useState<OpenState>({ phase: 'idle' });
  const [encoding, setEncodingState] = useState<LegacyEncoding>(initialEncoding);
  const setEncoding = useCallback((value: LegacyEncoding) => {
    setEncodingState(value);
    try {
      localStorage.setItem(ENCODING_STORAGE_KEY, value);
    } catch {
      /* storage unavailable; the session choice still applies */
    }
  }, []);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [detail, setDetail] = useState<HeroDetailDto | null>(null);
  const [editState, setEditState] = useState<HeroEditResponseDto | null>(null);
  const [detailError, setDetailError] = useState<FeedbackMessage | null>(null);
  // Bumped when the linked source map changes, to re-read hero map facts.
  const [mapLinkEpoch, setMapLinkEpoch] = useState(0);
  const [installation, setInstallation] = useState<InstallationDto | null>(null);
  const [installationError, setInstallationError] = useState<FeedbackMessage | null>(null);
  const [portraits, setPortraits] = useState<Map<number, PortraitDto>>(new Map());
  const [pending, setPending] = useState<PendingEditDto[]>([]);
  const [section, setSection] = useState<'heroes' | 'towns' | 'players' | 'creatures'>('heroes');
  const [heroTabRequest, setHeroTabRequest] = useState<{ heroId: number; tab: 'army'; nonce: number } | null>(null);
  const [players, setPlayers] = useState<PlayerDto[]>([]);
  const [selectedPlayerId, setSelectedPlayerId] = useState(0);
  const [playerEpoch, setPlayerEpoch] = useState(0);
  const [playerRequests, setPlayerRequests] = useState(0);
  const playerQueue = useRef(Promise.resolve());
  const [selectedTownId, setSelectedTownId] = useState<number | null>(null);
  const [townDetail, setTownDetail] = useState<TownDetailDto | null>(null);
  const [townEditState, setTownEditState] = useState<TownEditResponseDto | null>(null);
  const [townDetailError, setTownDetailError] = useState<FeedbackMessage | null>(null);
  const [saveState, setSaveState] = useState<
    | { kind: 'idle' }
    | { kind: 'saving'; asNewFile: boolean }
    | { kind: 'saved'; message: FeedbackMessage }
    | { kind: 'error'; message: FeedbackMessage }
  >({ kind: 'idle' });

  const refreshInstallation = useCallback(async () => {
    try {
      setInstallation(await getInstallationStatus());
      setInstallationError(null);
    } catch (error) {
      setInstallation(null);
      setInstallationError(errorMessage(error));
    }
  }, []);

  useEffect(() => {
    let active = true;
    getAppInfo().then(
      (appInfo) => { if (active) setInfo(appInfo); },
      () => { /* connection errors surface on open */ },
    );
    getInstallationStatus().then(
      (status) => { if (active) { setInstallation(status); setInstallationError(null); } },
      (error) => { if (active) setInstallationError(errorMessage(error)); },
    );
    return () => { active = false; };
  }, [refreshInstallation]);

  const handleChooseInstallation = useCallback(async () => {
    try {
      const chosen = await chooseInstallation();
      if (chosen) {
        setInstallation(chosen);
        setInstallationError(null);
      }
    } catch (error) {
      setInstallationError(errorMessage(error));
    }
  }, []);

  const handleOpen = useCallback(async () => {
    setOpenState({ phase: 'opening' });
    setSelectedId(null);
    setPlayers([]);
    setSelectedPlayerId(0);
    setDetail(null);
    setEditState(null);
    setSelectedTownId(null);
    setTownDetail(null);
    setTownEditState(null);
    setPending([]);
    setSaveState({ kind: 'idle' });
    try {
      const grant = await chooseSave();
      if (!grant) {
        setOpenState({ phase: 'idle' });
        return;
      }
      const summary = await openGrantedSave(grant.grantId);
      const heroes = await listHeroes();
      const towns = await listTowns();
      setPlayers(await listPlayers());
      setPlayerEpoch((n) => n + 1);
      setOpenState({ phase: 'ready', summary, heroes, towns });
      setPortraits(new Map());
      // Best-effort small portraits for the list; failures leave placeholders.
      getHeroPortraits(
        heroes.map((hero) => hero.id),
        'small',
      ).then(
        (results) => {
          const map = new Map<number, PortraitDto>();
          heroes.forEach((hero, index) => {
            const portrait = results[index];
            if (portrait) map.set(hero.id, portrait);
          });
          setPortraits(map);
        },
        () => { /* placeholders remain */ },
      );
    } catch (error) {
      setOpenState({ phase: 'error', message: errorMessage(error) });
    }
  }, []);

  // Map linking changes authored player facts, never resource pending state.
  useEffect(() => {
    if (mapLinkEpoch === 0) return;
    let active = true;
    playerQueue.current.then(() => listPlayers()).then(
      (updated) => { if (active) setPlayers(updated); },
      (error) => { if (active) setSaveState({ kind: 'error', message: errorMessage(error) }); },
    );
    return () => { active = false; };
  }, [mapLinkEpoch]);

  // Town detail + editable state follow the selected town.
  useEffect(() => {
    if (selectedTownId === null) {
      setTownDetail(null);
      setTownEditState(null);
      setTownDetailError(null);
      return;
    }
    let active = true;
    setTownDetailError(null);
    getTown(selectedTownId).then(
      (town) => { if (active) setTownDetail(town); },
      (error) => {
        if (active) {
          setTownDetail(null);
          setTownDetailError(errorMessage(error));
        }
      },
    );
    getTownEditState(selectedTownId).then(
      (state) => { if (active) setTownEditState(state); },
      () => { if (active) setTownEditState(null); },
    );
    return () => { active = false; };
  }, [selectedTownId, mapLinkEpoch]);

  const refreshTownList = useCallback(() => {
    listTowns().then((towns) => {
      setOpenState((current) => current.phase === 'ready' ? { ...current, towns } : current);
    }, () => { /* retain the last list if refresh fails */ });
  }, []);

  useEffect(() => {
    if (selectedId === null) {
      setDetail(null);
      setEditState(null);
      setDetailError(null);
      return;
    }
    let active = true;
    setDetailError(null);
    getHero(selectedId).then(
      (hero) => { if (active) setDetail(hero); },
      (error) => {
        if (active) {
          setDetail(null);
          setDetailError(errorMessage(error));
        }
      },
    );
    getHeroEditState(selectedId).then(
      (state) => { if (active) setEditState(state); },
      () => { if (active) setEditState(null); },
    );
    return () => { active = false; };
  }, [selectedId, mapLinkEpoch]);

  const refreshPending = useCallback(async () => {
    try {
      setPending(await getPendingEdits());
    } catch {
      setPending([]);
    }
  }, []);

  const refreshHeroList = useCallback(() => {
    listHeroes().then((heroes) => {
      setOpenState((current) => current.phase === 'ready' ? { ...current, heroes } : current);
    }, () => { /* retain the last list if refresh fails */ });
  }, []);

  const handleApplyEdit = useCallback(
    (request: HeroEditRequest) => {
      if (selectedId === null) return;
      applyHeroEdit(selectedId, request).then(
        (state) => {
          setEditState(state);
          if (request.army) refreshHeroList();
          refreshPending();
        },
        (error) => {
          setSaveState({ kind: 'error', message: errorMessage(error) });
        },
      );
    },
    [selectedId, refreshPending, refreshHeroList],
  );

  const handlePlayerEdit = useCallback((id: number, resource: number, value: number) => {
    setPlayerRequests((n) => n + 1);
    playerQueue.current = playerQueue.current.then(async () => {
      try {
        setPlayers(await applyPlayerEdit(id, resource, value));
        await refreshPending();
      } catch (error) {
        setSaveState({ kind: 'error', message: errorMessage(error) });
        setPlayerEpoch((n) => n + 1);
      } finally { setPlayerRequests((n) => n - 1); }
    });
  }, [refreshPending]);

  const handleTeamEdit = useCallback((id: number, team: number) => {
    setPlayerRequests((n) => n + 1);
    playerQueue.current = playerQueue.current.then(async () => {
      try {
        setPlayers(await applyTeamEdit(id, team));
        await refreshPending();
      } catch (error) {
        setSaveState({ kind: 'error', message: errorMessage(error) });
        setPlayerEpoch((n) => n + 1);
      } finally { setPlayerRequests((n) => n - 1); }
    });
  }, [refreshPending]);

  const handleTentEdit = useCallback((id: number, color: number, visited: boolean) => {
    setPlayerRequests((n) => n + 1);
    playerQueue.current = playerQueue.current.then(async () => {
      try {
        setPlayers(await applyTentEdit(id, color, visited));
        await refreshPending();
      } catch (error) {
        setSaveState({ kind: 'error', message: errorMessage(error) });
        setPlayerEpoch((n) => n + 1);
      } finally { setPlayerRequests((n) => n - 1); }
    });
  }, [refreshPending]);

  const handleRevertPlayer = useCallback(async (id: number) => {
    try {
      await playerQueue.current;
      setPlayers(await discardPlayerEdits(id));
      setPlayerEpoch((n) => n + 1);
      await refreshPending();
    } catch (error) { setSaveState({ kind: 'error', message: errorMessage(error) }); }
  }, [refreshPending]);

  const handleApplyTownEdit = useCallback(
    (request: TownEditRequest) => {
      if (selectedTownId === null) return;
      applyTownEdit(selectedTownId, request).then(
        (state) => {
          setTownEditState(state);
          if (request.army) refreshTownList();
          refreshPending();
        },
        (error) => {
          setSaveState({ kind: 'error', message: errorMessage(error) });
        },
      );
    },
    [selectedTownId, refreshPending, refreshTownList],
  );

  const handleDiscardTown = useCallback(() => {
    if (selectedTownId === null) return;
    discardTownEdits(selectedTownId).then(
      (remaining) => {
        setPending(remaining);
        refreshTownList();
        getTownEditState(selectedTownId).then(setTownEditState, () => setTownEditState(null));
      },
      (error) => setSaveState({ kind: 'error', message: errorMessage(error) }),
    );
  }, [selectedTownId, refreshTownList]);

  // From a town, jump to the hero owning its garrison (or visiting it), by
  // hero record ID, optionally straight to the Army tab. Pending edits live
  // in the backend session, so switching sections never drops them.
  const handleSelectHeroFromTown = useCallback((heroId: number, tab?: 'army') => {
    setSection('heroes');
    setSelectedId(heroId);
    if (tab) setHeroTabRequest((previous) => ({ heroId, tab, nonce: (previous?.nonce ?? 0) + 1 }));
  }, []);

  // Army edit of a town's garrisoned hero, made from the town screen.
  const handleApplyGarrisonHeroEdit = useCallback(
    (heroId: number, request: TownEditRequest) => {
      if (!request.army) return;
      applyHeroEdit(heroId, { army: request.army }).then(
        (state) => {
          if (selectedId === heroId) setEditState(state);
          refreshHeroList();
          refreshPending();
          if (selectedTownId !== null) {
            getTownEditState(selectedTownId).then(setTownEditState, () => setTownEditState(null));
          }
        },
        (error) => {
          setSaveState({ kind: 'error', message: errorMessage(error) });
        },
      );
    },
    [selectedId, selectedTownId, refreshHeroList, refreshPending],
  );

  const handleDiscardHero = useCallback(() => {
    if (selectedId === null) return;
    discardHeroEdits(selectedId).then(
      (remaining) => {
        setPending(remaining);
        refreshHeroList();
        getHeroEditState(selectedId).then(setEditState, () => setEditState(null));
      },
      (error) => setSaveState({ kind: 'error', message: errorMessage(error) }),
    );
  }, [selectedId, refreshHeroList]);

  const handleSetDismissal = useCallback((dismissed: boolean) => {
    if (selectedId === null) return;
    const heroId = selectedId;
    setHeroDismissal(heroId, dismissed).then(
      (state) => {
        setEditState(state);
        refreshPending();
      },
      (error) => setSaveState({ kind: 'error', message: errorMessage(error) }),
    );
  }, [selectedId, refreshPending]);

  const handleDiscardAll = useCallback(() => {
    discardAllEdits().then(
      (remaining) => {
        setPending(remaining);
        refreshHeroList();
        refreshTownList();
        listPlayers().then(setPlayers);
        setPlayerEpoch((n) => n + 1);
        setSaveState({ kind: 'idle' });
        if (selectedId !== null) {
          getHeroEditState(selectedId).then(setEditState, () => setEditState(null));
        }
        if (selectedTownId !== null) {
          getTownEditState(selectedTownId).then(setTownEditState, () => setTownEditState(null));
        }
      },
      (error) => setSaveState({ kind: 'error', message: errorMessage(error) }),
    );
  }, [selectedId, selectedTownId, refreshHeroList, refreshTownList]);

  // Why direct Save cannot overwrite the opened file (protected or unwritable), if so.
  const directSaveBlocked = openState.phase === 'ready'
    ? openState.summary.saveDisabledReason ?? null : null;

  const handleSave = useCallback(async (asNewFile: boolean) => {
    // Save stays clickable whenever there are changes; a source that cannot
    // be overwritten explains itself here instead of hiding behind a tooltip.
    if (!asNewFile && directSaveBlocked) {
      setSaveState({ kind: 'error', message: { code: 'save-disabled', message: directSaveBlocked.details } });
      return;
    }
    setSaveState({ kind: 'saving', asNewFile });
    try {
      const summary = await (asNewFile ? saveSaveAs() : save());
      if (!summary) {
        setSaveState({ kind: 'idle' });
        return;
      }
      const heroes = await listHeroes();
      const towns = await listTowns();
      setPlayers(await listPlayers());
      setPlayerEpoch((n) => n + 1);
      setOpenState({ phase: 'ready', summary, heroes, towns });
      setPending([]);
      setSaveState({
        kind: 'saved',
        message: { savedFile: summary.fileName },
      });
      if (selectedId !== null) {
        const [hero, state] = await Promise.all([
          getHero(selectedId),
          getHeroEditState(selectedId),
        ]);
        setDetail(hero);
        setEditState(state);
      }
      if (selectedTownId !== null) {
        const [town, state] = await Promise.all([
          getTown(selectedTownId),
          getTownEditState(selectedTownId),
        ]);
        setTownDetail(town);
        setTownEditState(state);
      }
    } catch (error) {
      // Edits stay pending; the original stays untouched.
      setSaveState({ kind: 'error', message: errorMessage(error) });
      refreshPending();
    }
  }, [selectedId, selectedTownId, refreshPending, directSaveBlocked]);
  // Native window caption mirrors the opened save and its dirty state.
  const fileName = openState.phase === 'ready' ? openState.summary.fileName : null;
  useEffect(() => {
    if (!isTauri()) return;
    const base = info ? `${info.name} v${info.version}` : 'H3Forge';
    const title = fileName ? `${base} - ${fileName}${pending.length > 0 ? ' *' : ''}` : base;
    getCurrentWindow().setTitle(title).catch(() => { /* caption update is best-effort */ });
  }, [info, fileName, pending.length]);

  const heroes = openState.phase === 'ready' ? openState.heroes : [];

  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">H3</span>
          <span>H3Forge</span>
        </div>
        <div className="header-actions">
          <Settings />
          <button type="button" onClick={handleChooseInstallation}>{t('ui.installation')}</button>
          <button type="button" disabled={openState.phase === 'opening' || pending.length > 0 || saveState.kind === 'saving' || playerRequests > 0} title={pending.length ? t('ui.saveOrDiscardChangesBeforeOpeningAnotherSave') : undefined} onClick={handleOpen}>
            {openState.phase === 'opening' ? t('ui.opening') : t('ui.openSave')}
          </button>
          <button type="button" onClick={() => handleSave(false)}
            disabled={pending.length === 0 || saveState.kind === 'saving' || playerRequests > 0}
            title={pending.length === 0
              ? openState.phase === 'ready' ? t('ui.noChangesToSaveEditASupportedFieldFirst') : t('ui.openASaveToBegin')
              : reasonText(directSaveBlocked) ?? t('ui.saveToTheCurrentFileWithABackup')}>
            {saveState.kind === 'saving' && !saveState.asNewFile ? t('ui.saving') : t('ui.save')}
          </button>
          <button type="button" onClick={() => handleSave(true)}
            disabled={pending.length === 0 || saveState.kind === 'saving' || playerRequests > 0}
            title={pending.length === 0
              ? openState.phase === 'ready' ? t('ui.noChangesToSaveEditASupportedFieldFirst') : t('ui.openASaveToBegin')
              : t('ui.saveChangesToANewFile')}>
            {saveState.kind === 'saving' && saveState.asNewFile ? t('ui.saving') : t('ui.saveAs')}
          </button>
          {pending.length > 0 && <>
            <span className="unsaved-indicator" role="status">{t('common.unsaved', { count: pending.length })}</span>
            <button type="button" className="secondary-button"
              onClick={handleDiscardAll} disabled={saveState.kind === 'saving' || playerRequests > 0}>{t('ui.discardChanges')}</button>
          </>}
        </div>
        {saveState.kind === 'saved' && <p className="toolbar-message save-status-ok" role="status">{<Feedback message={saveState.message} />}</p>}
        {saveState.kind === 'error' && <p className="toolbar-message error" role="alert">{<Feedback message={saveState.message} />}</p>}
      </header>
      <main className="overview-main">
        {openState.phase === 'idle' && (
          <section className="welcome" aria-labelledby="welcome-title">
            <p className="eyebrow">{t('ui.heroesOfMightAndMagicIiiHornOfThe')}</p>
            <h1 id="welcome-title">
              {t('ui.editYourHeroes')}<br />
              <span>{t('ui.yourChangesSafelyBackedUp')}</span>
            </h1>
            <p className="intro">{t('ui.openASavegameToBrowseAndEditHeroAttributes', { value1: info ? t('ui.connectedToValueValue', { value1: info.name, value2: info.version }) : '.' })}</p>
          </section>
        )}

        {openState.phase === 'error' && (
          <p className="error banner-error" role="alert">
            {<Feedback message={openState.message} />}
          </p>
        )}

        {openState.phase === 'ready' && (
          <>
            <SaveBanner
              summary={openState.summary}
              encoding={encoding}
              onEncodingChange={setEncoding}
              onMapLinkChanged={() => setMapLinkEpoch((epoch) => epoch + 1)}
            />
            <nav className="section-nav" aria-label={t('ui.saveSections')}>
              <button type="button" aria-current={section === 'heroes' ? 'page' : undefined}
                onClick={() => setSection('heroes')}>{t('ui.heroes')}</button>
              <button type="button" aria-current={section === 'towns' ? 'page' : undefined}
                onClick={() => setSection('towns')}>{t('ui.towns')}</button>
              <button type="button" aria-current={section === 'players' ? 'page' : undefined}
                onClick={() => setSection('players')}>{t('ui.players')}</button>
              <button type="button" aria-current={section === 'creatures' ? 'page' : undefined}
                onClick={() => setSection('creatures')}>{t('neutral.creatures')}</button>
            </nav>
            <div className="overview-columns" inert={saveState.kind === 'saving'}>
              {section === 'creatures' ? <CreaturesPanel epoch={`${playerEpoch}:${mapLinkEpoch}:${openState.summary.fileName}`} onChanged={refreshPending} onBusy={delta => setPlayerRequests(n => n + delta)} /> : section === 'players' ? <PlayersPanel players={players} selectedId={selectedPlayerId}
                onSelect={setSelectedPlayerId} onEdit={handlePlayerEdit} onTentEdit={handleTentEdit} onTeamEdit={handleTeamEdit} onRevert={handleRevertPlayer}
                epoch={playerEpoch} encoding={encoding} mapLinkEpoch={mapLinkEpoch} artworkVersion={installation?.archiveRoot ?? ''} busy={playerRequests > 0} /> : section === 'heroes' ? (
                <>
                  <HeroList
                    heroes={heroes}
                    encoding={encoding}
                    selectedId={selectedId}
                    portraits={portraits}
                    onSelect={setSelectedId}
                  />
                  <div className="editor-scroll">
                    {detailError && (
                      <p className="error banner-error" role="alert">{<Feedback message={detailError} />}</p>
                    )}
                    {detail && (
                      <HeroDetail
                        hero={detail}
                        encoding={encoding}
                        edit={editState}
                        onApplyEdit={handleApplyEdit}
                        onDiscardHero={handleDiscardHero}
                        onSetDismissal={handleSetDismissal}
                        tabRequest={heroTabRequest}
                      />
                    )}
                    {!detail && !detailError && (
                      <p className="scope-note column-placeholder">{t('ui.selectAHeroToViewAttributesAndSpellsUnsupported')}</p>
                    )}
                  </div>
                </>
              ) : (
                <>
                  <TownList
                    towns={openState.phase === 'ready' ? openState.towns : []}
                    encoding={encoding}
                    selectedId={selectedTownId}
                    onSelect={setSelectedTownId}
                  />
                  <div className="editor-scroll">
                    {townDetailError && (
                      <p className="error banner-error" role="alert">{<Feedback message={townDetailError} />}</p>
                    )}
                    {townDetail && (
                      <TownDetail
                        town={townDetail}
                        encoding={encoding}
                        edit={townEditState}
                        onApplyEdit={handleApplyTownEdit}
                        onDiscardTown={handleDiscardTown}
                        onSelectHero={handleSelectHeroFromTown}
                        onApplyHeroArmyEdit={handleApplyGarrisonHeroEdit}
                      />
                    )}
                    {!townDetail && !townDetailError && (
                      <p className="scope-note column-placeholder">{t('ui.selectATownToViewItsGarrisonBuildingsAnd')}</p>
                    )}
                  </div>
                </>
              )}
            </div>
          </>
        )}
        <details className="workspace-diagnostics">
          <summary>{t('ui.advancedDiagnostics')}</summary>
          {installation && <div className="installation-bar">
            <p>{t('ui.artworkValueValue', { value1: installation.hasBaseArtwork ? t('ui.available') : t('ui.notAvailable'), value2: installation.archiveRoot })}</p>
            <ul>{installation.archives.map((archive) => <li key={archive.name}>
              {archive.name}: {archive.state.kind === 'readable'
                ? t('ui.valueEntries', { value1: archive.state.entries })
                : `${archive.state.kind} (${archive.state.reason})`}
            </li>)}</ul>
          </div>}
          {installationError && <p>{<Feedback message={installationError} />}</p>}
          {pending.length > 0 && <ul className="pending-list">{pending.map((edit, index) =>
            <li key={index}>{edit.recordKind === 'creature' ? t('neutral.creatures') : edit.recordKind === 'player' ? t('ui.player') : edit.recordKind === 'town' ? t('ui.town') : t('ui.hero')} {decodeText(edit.recordName, encoding)} · {edit.field}: {edit.before} → {edit.after}</li>
          )}</ul>}
        </details>
      </main>

      <footer>
        H3Forge <span>{t('ui.saveKeepsABackupSaveAsCreatesANew')}</span>
      </footer>
    </div>
  );
}
