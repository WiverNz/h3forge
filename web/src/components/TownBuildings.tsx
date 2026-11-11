import { ReasonText, reasonText, type Reason } from '../i18n/reasons';
import { t } from '../i18n/index';
import { useEffect, useState } from 'react';
import { getBuildingIcon } from '../api/resources';
import type { PortraitDto, TownBuildingDto, TownBuildingsDto, TownEditRequest } from '../types/api';

type GroupKey = TownBuildingDto['group'];

const GROUPS: { key: GroupKey; label: string }[] = [
  { key: 'halls', get label() { return t('ui.halls'); } },
  { key: 'fortifications', get label() { return t('ui.fortifications'); } },
  { key: 'mageGuild', get label() { return t('ui.mageGuild'); } },
  { key: 'economy', get label() { return t('ui.economy'); } },
  { key: 'dwellings', get label() { return t('ui.creatureDwellings'); } },
  { key: 'special', get label() { return t('ui.factionStructures'); } },
  { key: 'other', get label() { return t('ui.other'); } },
];

/** Building IDs drawn as one upgrade chain when they sit in the same group. */
const PAIRS: [number, number][] = [
  [6, 20],
  [14, 15],
  [18, 19],
  [24, 25],
  ...Array.from({ length: 7 }, (_, level): [number, number] => [30 + level, 37 + level]),
];

/** Splits one group into upgrade chains, in building order. */
function chainsOf(group: GroupKey, items: TownBuildingDto[]): TownBuildingDto[][] {
  if (group === 'halls' || group === 'fortifications' || group === 'mageGuild') {
    return items.length > 0 ? [items] : [];
  }
  const byId = new Map(items.map((item) => [item.id, item]));
  const used = new Set<number>();
  const chains: TownBuildingDto[][] = [];
  for (const item of items) {
    if (used.has(item.id)) continue;
    const pair = PAIRS.find(([first]) => first === item.id);
    const second = pair ? byId.get(pair[1]) : undefined;
    used.add(item.id);
    if (second) {
      used.add(second.id);
      chains.push([item, second]);
    } else {
      chains.push([item]);
    }
  }
  return chains;
}

/**
 * Buildings of one town as picture cards, in the spirit of the map editor's
 * Buildings tab but showing the saved game's current state. Clicking a card
 * constructs or removes the building at once as a pending edit; the backend
 * adds the earlier steps of an upgrade chain and removes what depends on a
 * removed building. Locked cards show a one-line reason. No resources are
 * spent.
 */
export function TownBuildings({
  buildings,
  onApplyEdit,
}: {
  buildings: TownBuildingsDto | null;
  onApplyEdit: (request: TownEditRequest) => void;
}) {
  const [note, setNote] = useState<{ id: number; text: Reason } | null>(null);
  const townType = buildings?.townType;
  useEffect(() => setNote(null), [townType]);
  if (!buildings) {
    return (
      <p className="fineprint edit-blocked">{t('ui.buildingStateIsDecodedOnlyForHota4411')}</p>
    );
  }
  const shown = buildings.items.filter((item) => item.built || item.allowed || item.editable);
  return (
    <div className="town-buildings">
      {buildings.readOnlyReason && (
        <p className="fineprint edit-blocked">{<ReasonText reason={buildings.readOnlyReason} />}</p>
      )}
      <p className="building-legend">
        <span className="building-key building-key-built">{t('ui.built')}</span>
        <span className="building-key building-key-available">{t('ui.canBeBuilt')}</span>
        <span className="building-key building-key-unavailable">{t('ui.notAvailable')}</span>
        <span className="fineprint">{t('ui.clickToBuildOrRemoveChangesStayPendingUntil')}</span>
      </p>
      <p className="fineprint building-note" role="status">
        {note ? `${shown.find((item) => item.id === note.id)?.name ?? t('ui.building')}: ${reasonText(note.text)}.` : ' '}
      </p>
      {GROUPS.map((group) => {
        const chains = chainsOf(group.key, shown.filter((item) => item.group === group.key));
        if (chains.length === 0) return null;
        return (
          <section key={group.key} className="building-group" aria-label={group.label}>
            <h4>{group.label}</h4>
            <div className="building-chains">
              {chains.map((chain) => (
                <ol key={chain[0]?.id ?? -1} className="building-chain">
                  {chain.map((item) => (
                    <li key={item.id}>
                      <BuildingCard
                        item={item}
                        townType={buildings.townType}
                        onToggle={() => {
                          if (item.editable) {
                            setNote(null);
                            onApplyEdit({ building: { id: item.id, built: !item.built } });
                          } else {
                            setNote({ id: item.id, text: item.lockReason ?? t('ui.cannotBeChanged') });
                          }
                        }}
                      />
                    </li>
                  ))}
                </ol>
              ))}
            </div>
          </section>
        );
      })}
    </div>
  );
}

function BuildingCard({
  item,
  townType,
  onToggle,
}: {
  item: TownBuildingDto;
  townType: number;
  onToggle: () => void;
}) {
  const state = item.built ? 'built' : item.allowed ? 'available' : 'unavailable';
  const stateLabel = item.built ? t('ui.built') : item.allowed ? t('ui.canBeBuilt') : t('ui.notAvailable');
  return (
    <button
      type="button"
      className={[
        'building-card',
        `building-card-${state}`,
        item.editable ? '' : 'building-card-locked',
        item.dirty ? 'building-card-dirty' : '',
      ].filter(Boolean).join(' ')}
      aria-pressed={item.built}
      aria-disabled={!item.editable}
      aria-label={`${item.name}: ${stateLabel}${item.editable ? '' : t('ui.locked')}`}
      title={item.editable
        ? t('ui.valueValueClickToValue', { value1: item.name, value2: stateLabel, value3: item.built ? t('ui.remove') : t('ui.canBeBuilt') })
        : t('ui.valueValueLockedValue', { value1: item.name, value2: stateLabel, value3: reasonText(item.lockReason) ?? t('ui.cannotBeChanged') })}
      onClick={onToggle}
    >
      <BuildingPicture townType={townType} building={item.id} />
      <span className="building-card-name">{item.name}</span>
      <span className="building-card-state">
        {stateLabel}{!item.editable && <span className="building-card-lock">{t('ui.locked2')}</span>}
      </span>
    </button>
  );
}

/** Original Town Hall picture, or a neutral placeholder when none is verified. */
function BuildingPicture({ townType, building }: { townType: number; building: number }) {
  const key = `${townType}:${building}`;
  const [loaded, setLoaded] = useState<{ key: string; icon: PortraitDto } | null>(null);
  useEffect(() => {
    let active = true;
    getBuildingIcon(townType, building).then(
      (icon) => { if (active) setLoaded({ key, icon }); },
      () => { if (active) setLoaded(null); },
    );
    return () => { active = false; };
  }, [townType, building, key]);
  const icon = loaded?.key === key ? loaded.icon : null;
  if (!icon || !icon.resolved || !icon.pngBase64) {
    return <span className="building-picture building-picture-placeholder" aria-hidden="true" />;
  }
  return (
    <img className="building-picture" src={`data:image/png;base64,${icon.pngBase64}`}
      alt="" width={150} height={70} />
  );
}
