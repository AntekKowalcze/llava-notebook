

export type InputType =
  | "switch"
  | "button"
  | "text"
  | "select"
  | "number"
  | "info"

export type Setting = {
  id: string
  settingName: string
  label: string
  description: string
  currentValue: string
  inputType: InputType

  show: boolean // frontend only
}

export type Section = {
  id: string
  sectionName: string
  sectionSettings: Setting[]
  subsections?: Section[]

  show: boolean // frontend only
}

export type UserConfig = {
  sections: Section[]
}
