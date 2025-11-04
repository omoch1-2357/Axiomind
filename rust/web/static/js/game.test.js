// Frontend tests for interactive game controls
// These tests validate the poker table UI and betting controls

import {
  renderPokerTable,
  renderBettingControls,
  renderHandResult,
  formatCard,
  getCardColor,
  validateBetAmount,
  showValidationError,
} from './game.js';

describe('Poker Table UI', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  test('renders poker table with player positions', () => {
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 20000, position: 'button', is_active: true },
        { id: 1, stack: 20000, position: 'big_blind', is_active: false }
      ],
      board: [],
      pot: 0
    });

    const playerElements = container.querySelectorAll('.player-seat');
    expect(playerElements.length).toBe(2);
    expect(container.querySelector('.player-seat[data-player="0"]')).toBeTruthy();
    expect(container.querySelector('.player-seat[data-player="1"]')).toBeTruthy();
  });

  test('displays player hole cards for human player', () => {
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 20000, position: 'button', hole_cards: ['As', 'Kh'], is_active: true },
        { id: 1, stack: 20000, position: 'big_blind', is_active: false }
      ],
      board: [],
      pot: 0
    });

    const humanCards = container.querySelector('.player-seat[data-player="0"] .hole-cards');
    expect(humanCards.children.length).toBe(2);
    expect(humanCards.textContent).toContain('A♠');
    expect(humanCards.textContent).toContain('K♥');
  });

  test('hides opponent hole cards', () => {
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 20000, position: 'button', hole_cards: ['As', 'Kh'], is_active: true },
        { id: 1, stack: 20000, position: 'big_blind', is_active: false }
      ],
      board: [],
      pot: 0
    });

    const opponentCards = container.querySelector('.player-seat[data-player="1"] .hole-cards');
    expect(opponentCards.classList.contains('hidden')).toBe(true);
  });

  test('displays community cards on board', () => {
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 20000, position: 'button', is_active: true },
        { id: 1, stack: 20000, position: 'big_blind', is_active: false }
      ],
      board: ['Ah', 'Kd', 'Qc'],
      pot: 500
    });

    const boardCards = container.querySelector('.community-cards');
    expect(boardCards.children.length).toBe(3);
    expect(boardCards.textContent).toContain('A♥');
    expect(boardCards.textContent).toContain('K♦');
    expect(boardCards.textContent).toContain('Q♣');
  });

  test('displays pot size', () => {
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 19750, position: 'button', is_active: true },
        { id: 1, stack: 19800, position: 'big_blind', is_active: false }
      ],
      board: [],
      pot: 450
    });

    const potElement = container.querySelector('.pot-display');
    expect(potElement.textContent).toContain('450');
  });
});

describe('Betting Controls', () => {
  let container;
  let mockSubmit;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    mockSubmit = jest.fn();
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  test('renders available betting actions', () => {
    container.innerHTML = renderBettingControls({
      available_actions: [
        { type: 'fold', min_amount: null, max_amount: null },
        { type: 'check', min_amount: null, max_amount: null },
        { type: 'bet', min_amount: 100, max_amount: 2000 }
      ],
      current_player: 0
    });

    expect(container.querySelector('button[data-action="fold"]')).toBeTruthy();
    expect(container.querySelector('button[data-action="check"]')).toBeTruthy();
    expect(container.querySelector('button[data-action="bet"]')).toBeTruthy();
  });

  test('disables controls when not players turn', () => {
    container.innerHTML = renderBettingControls({
      available_actions: [
        { type: 'fold', min_amount: null, max_amount: null },
        { type: 'call', min_amount: null, max_amount: null }
      ],
      current_player: 1
    });

    const buttons = container.querySelectorAll('button');
    buttons.forEach(button => {
      expect(button.disabled).toBe(true);
    });
  });

  test('validates bet amount within min and max', () => {
    container.innerHTML = renderBettingControls({
      available_actions: [
        { type: 'bet', min_amount: 100, max_amount: 2000 }
      ],
      current_player: 0
    });

    const betInput = container.querySelector('input[name="bet-amount"]');
    const betButton = container.querySelector('button[data-action="bet"]');

    // Test below minimum
    betInput.value = '50';
    expect(validateBetAmount(betInput)).toBe(false);

    // Test within range
    betInput.value = '500';
    expect(validateBetAmount(betInput)).toBe(true);

    // Test above maximum
    betInput.value = '3000';
    expect(validateBetAmount(betInput)).toBe(false);
  });

  test('shows error message for invalid bet amount', () => {
    container.innerHTML = renderBettingControls({
      available_actions: [
        { type: 'bet', min_amount: 100, max_amount: 2000 }
      ],
      current_player: 0
    });

    const betInput = container.querySelector('input[name="bet-amount"]');
    betInput.value = '50';

    const error = showValidationError(betInput, 100, 2000);
    expect(error).toBeTruthy();
    expect(container.querySelector('.validation-error')).toBeTruthy();
  });

  test('attaches htmx attributes for action submission', () => {
    const sessionId = 'test-session-123';
    container.innerHTML = renderBettingControls({
      available_actions: [
        { type: 'fold', min_amount: null, max_amount: null },
        { type: 'call', min_amount: null, max_amount: null }
      ],
      current_player: 0,
      session_id: sessionId
    });

    const foldButton = container.querySelector('button[data-action="fold"]');
    expect(foldButton.getAttribute('hx-post')).toBe(`/api/sessions/${sessionId}/actions`);
    expect(foldButton.getAttribute('hx-vals')).toContain('Fold');
    expect(foldButton.getAttribute('hx-swap')).toBe('none');
  });
});

describe('Input Validation', () => {
  test('validateBetAmount accepts valid amounts', () => {
    const input = { value: '500', dataset: { min: '100', max: '2000' } };
    expect(validateBetAmount(input)).toBe(true);
  });

  test('validateBetAmount rejects non-numeric input', () => {
    const input = { value: 'abc', dataset: { min: '100', max: '2000' } };
    expect(validateBetAmount(input)).toBe(false);
  });

  test('validateBetAmount rejects negative amounts', () => {
    const input = { value: '-100', dataset: { min: '100', max: '2000' } };
    expect(validateBetAmount(input)).toBe(false);
  });

  test('validateBetAmount rejects empty input', () => {
    const input = { value: '', dataset: { min: '100', max: '2000' } };
    expect(validateBetAmount(input)).toBe(false);
  });
});

describe('Card Rendering', () => {
  test('formatCard converts short notation to Unicode', () => {
    expect(formatCard('As')).toBe('A♠');
    expect(formatCard('Kh')).toBe('K♥');
    expect(formatCard('Qd')).toBe('Q♦');
    expect(formatCard('Jc')).toBe('J♣');
    expect(formatCard('Tc')).toBe('10♣');
    expect(formatCard('9s')).toBe('9♠');
  });

  test('getCardColor returns correct color for suit', () => {
    expect(getCardColor('s')).toBe('black');
    expect(getCardColor('c')).toBe('black');
    expect(getCardColor('h')).toBe('red');
    expect(getCardColor('d')).toBe('red');
  });
});

describe('Hand Result Display', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  test('displays hand result with winner information', () => {
    container.innerHTML = renderHandResult({
      winner: 0,
      winner_name: 'You',
      amount: 500,
      hand_description: 'Pair of Aces',
      showdown: {
        player_0_cards: ['As', 'Ah'],
        player_1_cards: ['Kh', 'Qd']
      }
    });

    expect(container.querySelector('.hand-result')).toBeTruthy();
    expect(container.textContent).toContain('You');
    expect(container.textContent).toContain('500');
    expect(container.textContent).toContain('Pair of Aces');
  });

  test('displays showdown cards for both players', () => {
    container.innerHTML = renderHandResult({
      winner: 0,
      winner_name: 'You',
      amount: 500,
      hand_description: 'Pair of Aces',
      showdown: {
        player_0_cards: ['As', 'Ah'],
        player_1_cards: ['Kh', 'Qd']
      }
    });

    const showdownSection = container.querySelector('.showdown-cards');
    expect(showdownSection).toBeTruthy();
    expect(showdownSection.textContent).toContain('A♠');
    expect(showdownSection.textContent).toContain('A♥');
    expect(showdownSection.textContent).toContain('K♥');
    expect(showdownSection.textContent).toContain('Q♦');
  });

  test('displays split pot result', () => {
    container.innerHTML = renderHandResult({
      winner: null,
      amount: 500,
      hand_description: 'Split Pot',
      split: true
    });

    expect(container.querySelector('.hand-result')).toBeTruthy();
    expect(container.textContent).toContain('Split Pot');
    expect(container.textContent).toContain('500');
  });

  test('displays fold result without showdown', () => {
    container.innerHTML = renderHandResult({
      winner: 0,
      winner_name: 'You',
      amount: 200,
      hand_description: 'Opponent folded',
      fold: true
    });

    expect(container.querySelector('.hand-result')).toBeTruthy();
    expect(container.textContent).toContain('Opponent folded');
    expect(container.querySelector('.showdown-cards')).toBeFalsy();
  });

  test('includes continue button', () => {
    container.innerHTML = renderHandResult({
      winner: 0,
      winner_name: 'You',
      amount: 500,
      hand_description: 'Pair of Aces'
    });

    const continueButton = container.querySelector('.continue-button');
    expect(continueButton).toBeTruthy();
    expect(continueButton.textContent).toContain('Continue');
  });
});

describe('Active Player Highlighting', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  test('highlights active player seat', () => {
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 20000, position: 'button', is_active: true },
        { id: 1, stack: 20000, position: 'big_blind', is_active: false }
      ],
      board: [],
      pot: 0,
      current_player: 0
    });

    const activeSeat = container.querySelector('.player-seat[data-player="0"]');
    expect(activeSeat.getAttribute('data-active')).toBe('true');
  });

  test('removes highlight from inactive player', () => {
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 20000, position: 'button', is_active: false },
        { id: 1, stack: 20000, position: 'big_blind', is_active: true }
      ],
      board: [],
      pot: 0,
      current_player: 1
    });

    const inactiveSeat = container.querySelector('.player-seat[data-player="0"]');
    expect(inactiveSeat.getAttribute('data-active')).toBeFalsy();
  });
});

describe('Real-time State Updates', () => {
  let container;

  beforeEach(() => {
    container = document.createElement('div');
    container.id = 'table';
    document.body.appendChild(container);
  });

  afterEach(() => {
    document.body.removeChild(container);
  });

  test('updates pot display when state changes', () => {
    // Initial state
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 20000, position: 'button', is_active: true },
        { id: 1, stack: 20000, position: 'big_blind', is_active: false }
      ],
      board: [],
      pot: 150
    });

    expect(container.querySelector('.pot-amount').textContent).toBe('150');

    // Updated state
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 19500, position: 'button', is_active: true },
        { id: 1, stack: 19500, position: 'big_blind', is_active: false }
      ],
      board: [],
      pot: 1000
    });

    expect(container.querySelector('.pot-amount').textContent).toBe('1,000');
  });

  test('updates player stacks when state changes', () => {
    // Initial state
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 20000, position: 'button', is_active: true },
        { id: 1, stack: 20000, position: 'big_blind', is_active: false }
      ],
      board: [],
      pot: 0
    });

    let player0Stack = container.querySelector('.player-seat[data-player="0"] .player-stack');
    expect(player0Stack.textContent).toContain('20,000');

    // Updated state
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 19500, position: 'button', is_active: true },
        { id: 1, stack: 20500, position: 'big_blind', is_active: false }
      ],
      board: [],
      pot: 0
    });

    player0Stack = container.querySelector('.player-seat[data-player="0"] .player-stack');
    expect(player0Stack.textContent).toContain('19,500');
  });

  test('adds community cards progressively', () => {
    // Pre-flop
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 20000, position: 'button', is_active: true },
        { id: 1, stack: 20000, position: 'big_blind', is_active: false }
      ],
      board: [],
      pot: 150
    });

    expect(container.querySelectorAll('.community-cards .card').length).toBe(0);

    // Flop
    container.innerHTML = renderPokerTable({
      players: [
        { id: 0, stack: 19750, position: 'button', is_active: true },
        { id: 1, stack: 19750, position: 'big_blind', is_active: false }
      ],
      board: ['Ah', 'Kd', 'Qc'],
      pot: 500
    });

    expect(container.querySelectorAll('.community-cards .card').length).toBe(3);
  });
});
