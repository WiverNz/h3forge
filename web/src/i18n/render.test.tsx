import assert from 'node:assert/strict';
import { test } from 'node:test';
import { renderToStaticMarkup } from 'react-dom/server';
import { App } from '../App';
import { ArmyEditor } from '../components/ArmyEditor';
import { PlayersPanel } from '../components/PlayersPanel';
import { SaveBanner } from '../components/SaveBanner';
import { TownBuildings } from '../components/TownBuildings';
import { CoordinatesEditor } from '../components/CoordinatesEditor';
import { Feedback } from './Feedback';
import { ReasonText } from './reasons';
import { i18n, t } from './index';
import { languages } from './locale';
import type { ArmySlotDto, PlayerDto, SaveSummaryDto } from '../types/api';

test('four-language shell, editor and error rendering keeps identities, classes and dirty markers', async () => {
  const stack: ArmySlotDto = { creatureId: 0, count: 1234, name: '<Custom & Иван>', empty: false, editable: true };
  const player = {
    id: 0, color: 0, name: 'Red', status: 'active', statusReason: 'technical detail',
    heroCount: 1, townCount: 1, resources: [1, 2, 3, 4, 5, 6, 7], original: [1, 2, 3, 4, 5, 6, 7],
    team: null, originalTeam: null, teamCount: 0, teamOptions: [], teamState: 'unsupported', teamDirty: false,
    tentVisits: Array(8).fill(false), originalTentVisits: Array(8).fill(false), dirtyTents: [], dirtyResources: [], readOnlyReason: null, details: { startingHero: null, startingTown: null, team: null, eligibility: null,
      allies: null, enemies: null, mapSize: null, obelisks: null },
  } as unknown as PlayerDto;
  for (const language of languages) {
    await i18n.changeLanguage(language);
    const shell = renderToStaticMarkup(<App />);
    const summary: SaveSummaryDto = {
      fileName: 'native.GM2', familyTitle: 'HotA', major: 44, minor: 11,
      containerKind: 'regular', compatibility: 'readable', heroesSupported: true,
      mapName: null, mapDescription: null, heroCount: 0, confirmedCount: 0,
      saveDisabledReason: null,
      diagnostics: [{ code: 'native-gzip-crc-mismatch', message: 'Technical CRC values' }],
    };
    const banner = renderToStaticMarkup(<SaveBanner summary={summary} encoding="auto"
      onEncodingChange={() => {}} onMapLinkChanged={() => {}} />);
    assert.ok(banner.includes(t('teams.nativeCrc')));
    assert.ok(banner.includes('role="status"'));
    const cleanBanner = renderToStaticMarkup(<SaveBanner summary={{ ...summary, diagnostics: [] }}
      encoding="auto" onEncodingChange={() => {}} onMapLinkChanged={() => {}} />);
    assert.ok(!cleanBanner.includes(t('teams.nativeCrc')));
    assert.ok(shell.includes(t('common.settings')));
    assert.ok(shell.includes(t('common.theme')));
    assert.ok(shell.includes(t('common.themeLight')));
    assert.ok(shell.includes(t('common.themeDark')));
    assert.ok(shell.includes(t('common.themeSystem')));
    assert.ok(shell.includes(t('ui.openSave')));
    const army = renderToStaticMarkup(<ArmyEditor slots={[stack]} choices={[]} readOnlyReason={null}
      dirtyFields={new Set(['Army slot 1'])} onApplyEdit={() => { throw new Error('render must not edit'); }} />);
    assert.ok(army.includes('army-stack-dirty'));
    assert.ok(army.includes('value="1234"'));
    const players = renderToStaticMarkup(<PlayersPanel players={[player]} selectedId={0} onSelect={() => {}}
      onTentEdit={() => { throw new Error("render must not edit"); }} onTeamEdit={() => { throw new Error('render must not edit'); }} onEdit={() => { throw new Error('render must not edit'); }} onRevert={() => {}}
      epoch={0} artworkVersion="" busy={false} encoding="auto" mapLinkEpoch={0} />);
    assert.ok(players.includes(t('tents.title')));
    assert.equal((players.match(/<select /g) ?? []).length, 10);
    assert.ok(/<select id="current-player-team" disabled="" aria-describedby="team-edit-note"><option value="unknown" selected="">/.test(players));
    assert.ok(players.includes(t('teams.current')));
    assert.ok(players.includes(t('teams.unsupported')));
    const teamPlayers = [
      { ...player, team: 2, originalTeam: 1, teamCount: 2, teamOptions: [1, 2], teamState: 'editable', teamDirty: true },
      { ...player, id: 1, color: 1, team: 1, originalTeam: 1, teamCount: 2, teamOptions: [1], teamState: 'editable' },
      { ...player, id: 2, color: 2, team: 2, originalTeam: 2, teamCount: 2, teamOptions: [1, 2], teamState: 'editable' },
    ] as PlayerDto[];
    const teamPanel = (selected: number) => renderToStaticMarkup(<PlayersPanel players={teamPlayers} selectedId={selected}
      onSelect={() => {}} onTentEdit={() => { throw new Error('must not edit'); }} onTeamEdit={() => { throw new Error('must not edit'); }}
      onEdit={() => { throw new Error('must not edit'); }} onRevert={() => {}}
      epoch={0} artworkVersion="" busy={false} encoding="auto" mapLinkEpoch={0} />);
    const moved = teamPanel(0);
    assert.ok(moved.includes('<select id="current-player-team" aria-describedby="team-edit-note" class="field-dirty"'));
    assert.ok(moved.includes(`<option value="2" selected="">${t('ui.teamValue', { value1: 2 })} (`));
    assert.ok(moved.includes(`${t('teams.current')} *`));
    assert.ok(moved.includes(t('teams.editable')));
    // A player who is alone in a team keeps a disabled selector.
    assert.ok(/<select id="current-player-team" disabled="" aria-describedby="team-edit-note"[^>]*><option value="1" selected="">/.test(teamPanel(1)));
    assert.ok(teamPanel(1).includes('<option value="2" disabled="">'));
    const linkedPlayer = { ...player, details: { ...player.details, mapLinked: true, team: 2, allies: [1], enemies: [4] } };
    const linked = renderToStaticMarkup(<PlayersPanel players={[linkedPlayer]} selectedId={0}
      onSelect={() => {}} onTentEdit={() => { throw new Error('must not edit'); }} onTeamEdit={() => { throw new Error('must not edit'); }}
      onEdit={() => { throw new Error('must not edit'); }} onRevert={() => {}}
      epoch={0} artworkVersion="" busy={false} encoding="auto" mapLinkEpoch={1} />);
    assert.ok(/<select id="current-player-team" disabled="" aria-describedby="team-edit-note"><option value="unknown" selected="">/.test(linked));
    assert.ok(linked.includes(t('ui.mapTeam')));
    assert.ok(linked.includes(t('ui.teamValue', { value1: 2 })));
    assert.ok(linked.includes(t('ui.mapAllies')));
    assert.ok(linked.includes(t('ui.mapEnemies')));
    assert.ok(players.includes(t('lists.sort')));
    assert.ok(players.includes(t('lists.filter')));
    assert.ok(players.includes(t('tents.notVisited')));
    assert.ok(players.includes('id="tab-Resources"'));
    assert.ok(players.includes('id="player-Resources"'));
    assert.ok(players.includes(t('common.resources')));
    const town = renderToStaticMarkup(<TownBuildings buildings={null} onApplyEdit={() => {}} />);
    const coordinates = renderToStaticMarkup(<CoordinatesEditor value={[230,52,0]} blocked={null} dirty
      onApply={() => { throw new Error('render must not edit'); }} />);
    assert.ok(coordinates.includes(t('ui.applyCoordinates')));
    assert.ok(coordinates.includes(t('ui.heroRelocationPolicy')));
    assert.ok(coordinates.includes('value="230"'));
    assert.ok(coordinates.includes('readOnly=""'));
    assert.ok(!coordinates.includes('<fieldset class="hero-coordinates field-dirty" disabled'));
    const blockedCoordinates=renderToStaticMarkup(<CoordinatesEditor value={null}
      blocked={{code:'read-only',details:'Unsupported position'}} dirty={false} onApply={() => {}} />);
    assert.ok(blockedCoordinates.includes('disabled=""'));
    assert.ok(town.includes(t('ui.buildingStateIsDecodedOnlyForHota4411')));
    const feedback = renderToStaticMarkup(<Feedback message={{ savedFile: '<Custom & Иван>' }} />);
    assert.ok(feedback.includes('&lt;Custom &amp; Иван&gt;'));
    const reason = renderToStaticMarkup(<ReasonText reason={{ code: 'read-only', details: '<unknown bytes>' }} />);
    assert.ok(reason.includes(t('ui.readOnly')));
    assert.ok(reason.includes('&lt;unknown bytes&gt;'));
    for (const html of [shell, army, players, town, feedback, reason, coordinates, blockedCoordinates]) {
      assert.ok(!html.includes('[object Object]'));
      assert.ok(!html.includes('{{'));
      assert.ok(!html.includes('ui.'));
    }
  }
  await i18n.changeLanguage('en');
});

// This is a compile-time check only; an unknown key must not become valid.
test('compact garrison keeps seven accessible editable slots, values and dirty markers', () => {
  const slots: ArmySlotDto[] = Array.from({ length: 7 }, (_, i) => ({ creatureId: i, count: 100 + i,
    name: `Creature ${i}`, empty: false, editable: true }));
  const html = renderToStaticMarkup(<ArmyEditor compact slots={slots}
    choices={slots.map(s => ({ id: s.creatureId, name: s.name! }))} readOnlyReason={null}
    dirtyPrefix="Garrison slot" dirtyFields={new Set(['Garrison slot 7'])}
    onApplyEdit={() => { throw new Error('render must not edit'); }} />);
  assert.ok(html.includes('army-grid-compact'));
  assert.equal((html.match(/role="group"/g) ?? []).length, 7);
  assert.equal((html.match(/type="number"/g) ?? []).length, 7);
  assert.equal((html.match(/aria-haspopup="listbox"/g) ?? []).length, 7);
  assert.equal((html.match(/army-stack-dirty/g) ?? []).length, 1);
  assert.ok(!html.includes('army-field-caption'));
  assert.ok(!html.includes(t('ui.armyChangesBecomePendingEditsImmediatelyNothingIsWritten')));
  for (let i = 0; i < 7; i++) assert.ok(html.includes(`value="${100 + i}"`));
});

if (false) {
  // @ts-expect-error nonexistent keys must be rejected
  t('missing.key');
}
