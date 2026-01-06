interface ButtonProps {
  children: React.ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  type?: 'button' | 'submit' | 'reset';
}

export function Button({ children, onClick, disabled, type = 'button' }: ButtonProps) {
  return (
    <button type={type} onClick={onClick} disabled={disabled}>
      {children}
    </button>
  );
}
