

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
  options: string[] | null
  buttonLabel: string | null


  show: boolean // frontend only
}

export type Section = {
  id: string
  sectionName: string
  sectionSettings: Setting[]
  subsections?: Section[]

  scrollPosition: number
  show: boolean // frontend only
}

export type UserConfig = {
  sections: Section[]
}
