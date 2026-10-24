import { createSignal } from 'solid-js';
import { render, screen, fireEvent } from '@solidjs/testing-library';
import { expect, test } from 'vitest';

function Counter() {
  const [n, setN] = createSignal(0);
  return <button onClick={() => setN(n() + 1)}>count {n()}</button>;
}

test('renders and updates a Solid 2 component', async () => {
  render(() => <Counter />);
  const button = screen.getByRole('button');
  expect(button.textContent).toBe('count 0');
  fireEvent.click(button);
  expect(await screen.findByText('count 1')).toBeTruthy();
});
