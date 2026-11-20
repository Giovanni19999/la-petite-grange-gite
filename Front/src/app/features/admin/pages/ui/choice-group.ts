import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';

export interface Choice<T extends string | number> {
  value: T;
  label: string;
}

let nextId = 0;

/** Segmented choice built on native radio buttons (keyboard and screen reader friendly). */
@Component({
  selector: 'app-choice-group',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <fieldset class="flex flex-col gap-2">
      <legend class="mb-2 text-sm font-medium">{{ legend() }}</legend>
      <div class="flex flex-wrap gap-1 rounded-md bg-muted p-1">
        @for (choice of choices(); track choice.value) {
          <label
            class="flex-1 cursor-pointer rounded px-2 py-1.5 text-center text-sm whitespace-nowrap has-checked:bg-background has-checked:font-medium has-checked:shadow-sm has-focus-visible:ring-2 has-focus-visible:ring-ring"
          >
            <input
              class="sr-only"
              type="radio"
              [name]="name"
              [value]="choice.value"
              [checked]="choice.value === value()"
              (change)="changed.emit(choice.value)"
            />
            {{ choice.label }}
          </label>
        }
      </div>
    </fieldset>
  `,
})
export class ChoiceGroup<T extends string | number> {
  readonly legend = input.required<string>();
  readonly choices = input.required<readonly Choice<T>[]>();
  readonly value = input.required<T>();
  readonly changed = output<T>();

  protected readonly name = `choice-${nextId++}`;
}
