import { ReasonText } from '../i18n/reasons';
import { t } from '../i18n/index';
import type { GuildLevelDto, TownEditRequest, TownGuildDto } from '../types/api';
import { SpellCombobox } from './SpellCombobox';

/**
 * Mage Guild editor: the spells the saved game currently offers at each
 * guild level, never the source map's must/may masks. Picking a spell
 * replaces or adds it and the remove button drops it; every change is an
 * absolute spell list for that level and becomes a pending edit at once.
 */
export function TownGuild({
  guild,
  onApplyEdit,
}: {
  guild: TownGuildDto | null;
  onApplyEdit: (request: TownEditRequest) => void;
}) {
  if (!guild) {
    return (
      <p className="fineprint edit-blocked">{t('ui.mageGuildStateIsDecodedOnlyForHota44')}</p>
    );
  }
  return (
    <div className="town-guild">
      {guild.readOnlyReason && (
        <p className="fineprint edit-blocked">{<ReasonText reason={guild.readOnlyReason} />}</p>
      )}
      <p className="fineprint">{t('ui.builtLevelValueOfValuePossibleInThisTown', { value1: guild.builtLevel, value2: guild.maxLevel, value3: guild.hasLibrary ? t('messages.library') : '' })}</p>
      {guild.levels.map((level) => (
        <GuildLevel key={level.level} level={level} guild={guild} onApplyEdit={onApplyEdit} />
      ))}
    </div>
  );
}

function GuildLevel({
  level,
  guild,
  onApplyEdit,
}: {
  level: GuildLevelDto;
  guild: TownGuildDto;
  onApplyEdit: (request: TownEditRequest) => void;
}) {
  const ids = level.spells.map((spell) => spell.id);
  const commit = (spells: number[]) => onApplyEdit({ guild: { level: level.level, spells } });
  const candidates = guild.spellChoices.filter(
    (choice) => choice.level === level.level && !choice.banned,
  );
  const unused = candidates.filter((choice) => !ids.includes(choice.id));
  const status = !level.available ? t('ui.notAvailableHere') : level.built ? t('ui.built') : t('ui.notBuilt');
  return (
    <section
      className={`guild-level${level.dirty ? ' guild-level-dirty' : ''}${level.available ? '' : ' guild-level-unavailable'}`}
      aria-label={t('ui.mageGuildLevelValue', { value1: level.level })}
    >
      <header className="guild-level-head">
        <h4>{t('ui.levelValue', { value1: level.level })}</h4>
        <span className={`guild-level-status guild-level-${level.built ? 'built' : 'idle'}`}>{status}</span>
        <span className="fineprint">{level.spells.length} {' '}{t('ui.of')}{' '}{level.capacity} {t('ui.spells2')}</span>
      </header>
      <ul className="guild-spells">
        {level.spells.map((spell, index) => (
          <li key={spell.id} className="guild-spell">
            <SpellCombobox
              value={spell.id}
              valueName={spell.name ?? t('ui.spellValue2', { value1: spell.id })}
              placeholder={t('ui.replaceSpell')}
              disabled={!level.editable}
              choices={unused}
              onChange={(id) => commit(ids.map((old, at) => (at === index ? id : old)))}
            />
            {level.editable && (
              <button
                type="button"
                className="guild-spell-remove"
                aria-label={t('ui.removeValue', { value1: spell.name ?? `spell ${spell.id}` })}
                title={t('ui.removeFromThisGuildLevel')}
                onClick={() => commit(ids.filter((_, at) => at !== index))}
              >
                ×
              </button>
            )}
          </li>
        ))}
        {level.editable && level.spells.length < level.capacity && (
          <li className="guild-spell guild-spell-add">
            <SpellCombobox
              placeholder={unused.length === 0 ? t('ui.noFurtherSpellsAllowed') : t('ui.addSpell')}
              choices={unused}
              onChange={(id) => commit([...ids, id])}
            />
          </li>
        )}
        {level.spells.length === 0 && !level.editable && (
          <li className="fineprint">{t('ui.noSpellsStored')}</li>
        )}
      </ul>
    </section>
  );
}
