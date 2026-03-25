export enum InputTypes {
  Text = 'text', // Standard text
  Password = 'password', // Masked characters
  Email = 'email', // Validates email format
  Number = 'number', // Numeric keypad on mobile
  Search = 'search', // Adds 'x' clear button in some browsers
  Tel = 'tel', // Phone keypad on mobile
  Url = 'url', // URL keypad on mobile
  Date = 'date', // Date picker (styles vary by browser)
  Time = 'time', // Time picker
  DatetimeLocal = 'datetime-local',
  Month = 'month',
  Week = 'week',
}
export interface Input {
  name: string;
  placeholder: string;
  type: InputTypes;
  showValidation?: boolean;
}
