/**
 * Axiomind Poker - Interactive Game Controls
 * Handles poker table rendering, betting controls, and htmx integration
 */

/**
 * Card formatting utilities
 */

/**
 * Normalize card to object format
 * Accepts both string format ("9d", "Ah") and object format ({suit: "Diamonds", rank: "Nine"})
 */
function normalizeCard(card) {
  if (!card) return null;

  // If already an object with suit and rank, return as-is
  if (typeof card === 'object' && card.suit && card.rank) {
    return card;
  }

  // If string format, parse it
  if (typeof card === 'string' && card.length >= 2) {
    const rank = card.slice(0, -1).toUpperCase();
    const suitChar = card.slice(-1).toLowerCase();

    const rankMap = {
      '2': 'Two', '3': 'Three', '4': 'Four', '5': 'Five',
      '6': 'Six', '7': 'Seven', '8': 'Eight', '9': 'Nine',
      'T': 'Ten', 'J': 'Jack', 'Q': 'Queen', 'K': 'King', 'A': 'Ace'
    };

    const suitMap = {
      'c': 'Clubs',
      'd': 'Diamonds',
      'h': 'Hearts',
      's': 'Spades'
    };

    return {
      rank: rankMap[rank] || rank,
      suit: suitMap[suitChar] || suitChar
    };
  }

  return null;
}

function formatCard(card) {
  const normalized = normalizeCard(card);
  if (!normalized) return '';

  const rankMap = {
    'Two': '2', 'Three': '3', 'Four': '4', 'Five': '5',
    'Six': '6', 'Seven': '7', 'Eight': '8', 'Nine': '9',
    'Ten': '10', 'Jack': 'J', 'Queen': 'Q', 'King': 'K', 'Ace': 'A'
  };

  const suitMap = {
    'Clubs': '♣',
    'Diamonds': '♦',
    'Hearts': '♥',
    'Spades': '♠'
  };

  const displayRank = rankMap[normalized.rank] || normalized.rank;
  const displaySuit = suitMap[normalized.suit] || normalized.suit;

  return displayRank + displaySuit;
}

function getCardColor(card) {
  const normalized = normalizeCard(card);
  if (!normalized) return 'black';

  const suit = normalized.suit;
  return (suit === 'Hearts' || suit === 'Diamonds') ? 'red' : 'black';
}

function renderCard(card) {
  const formatted = formatCard(card);
  const color = getCardColor(card);
  return `<span class="card card-${color}">${formatted}</span>`;
}

/**
 * Poker table rendering
 */
function renderPokerTable(state) {
  const { players, board, pot, current_player, session_id } = state;

  const playerSeats = players.map(player => {
    const isActive = player.is_active || (current_player === player.id);
    const holeCards = player.hole_cards || [];
    const showCards = player.id === 0 && holeCards.length > 0;

    return `
      <div class="player-seat" data-player="${player.id}" ${isActive ? 'data-active="true"' : ''}>
        <div class="player-info">
          <div class="player-name">${player.id === 0 ? 'You' : 'Opponent'}</div>
          <div class="player-position">${player.position.replace('_', ' ')}</div>
          <div class="player-stack">Stack: ${player.stack.toLocaleString()}</div>
        </div>
        <div class="hole-cards ${showCards ? '' : 'hidden'}">
          ${showCards ? holeCards.map(card => renderCard(card)).join('') : `
            <span class="card card-back">🂠</span>
            <span class="card card-back">🂠</span>
          `}
        </div>
      </div>
    `;
  }).join('');

  const communityCards = board.map(card => renderCard(card)).join('');

  return `
    <div class="poker-table" data-session="${session_id || ''}">
      <div class="table-layout">
        <div class="players-container">
          ${playerSeats}
        </div>
        <div class="table-center">
          <div class="community-cards">
            ${communityCards || '<span class="placeholder">Community cards will appear here</span>'}
          </div>
          <div class="pot-display">
            Pot: <span class="pot-amount">${pot.toLocaleString()}</span>
          </div>
        </div>
      </div>
    </div>
  `;
}

/**
 * Betting controls rendering
 */
function renderBettingControls(state) {
  const { available_actions, current_player, session_id } = state;
  const isPlayerTurn = current_player === 0;

  if (!available_actions || available_actions.length === 0) {
    return '<div class="betting-controls"><p class="info">Waiting for game to start...</p></div>';
  }

  const actionButtons = available_actions.map(action => {
    const { type, min_amount, max_amount } = action;

    if (type === 'bet' || type === 'raise') {
      const minValue = typeof min_amount === 'number' && !Number.isNaN(min_amount) ? min_amount : 0;
      const hasMax = typeof max_amount === 'number' && !Number.isNaN(max_amount) && Number.isFinite(max_amount);
      const maxAttribute = hasMax ? `max="${max_amount}"` : '';
      const dataMax = hasMax ? max_amount : '';
      return `
        <div class="action-group">
          <input
            type="number"
            name="bet-amount"
            class="bet-input"
            placeholder="Amount"
            min="${minValue}"
            ${maxAttribute}
            data-min="${minValue}"
            data-max="${dataMax}"
            ${!isPlayerTurn ? 'disabled' : ''}
            oninput="validateBetAmountInput(this)"
          />
          <button
            class="action-btn action-${type}"
            data-action="${type}"
            ${!isPlayerTurn ? 'disabled' : ''}
            hx-post="/api/sessions/${session_id}/actions"
            hx-vals='js:{action: getBetAction("${type}")}'
            hx-swap="none"
            hx-trigger="click"
            onclick="return validateBeforeSubmit(this)"
          >
            ${type.charAt(0).toUpperCase() + type.slice(1)}
          </button>
          <div class="bet-range-hint">Min: ${minValue} - Max: ${hasMax ? max_amount : 'No limit'}</div>
          <div class="validation-error" style="display: none;"></div>
        </div>
      `;
    } else {
      const actionJson = JSON.stringify(getActionPayload(type, null));
      return `
        <button
          class="action-btn action-${type}"
          data-action="${type}"
          ${!isPlayerTurn ? 'disabled' : ''}
          hx-post="/api/sessions/${session_id}/actions"
          hx-vals='{"action": ${actionJson}}'
          hx-swap="none"
          hx-trigger="click"
        >
          ${type.charAt(0).toUpperCase() + type.slice(1)}
        </button>
      `;
    }
  }).join('');

  return `
    <div class="betting-controls ${isPlayerTurn ? 'active' : 'inactive'}">
      <div class="controls-header">
        ${isPlayerTurn ? '<p class="your-turn">Your turn</p>' : '<p class="waiting">Waiting for opponent...</p>'}
      </div>
      <div class="actions-container">
        ${actionButtons}
      </div>
    </div>
  `;
}

/**
 * Input validation
 */
function validateBetAmount(input) {
  const value = parseFloat(input.value);
  const parsedMin = parseFloat(input.dataset.min);
  const min = Number.isNaN(parsedMin) ? 0 : parsedMin;
  const rawMax = input.dataset.max;
  const parsedMax = parseFloat(rawMax);
  const max = (rawMax === '' || Number.isNaN(parsedMax)) ? Number.POSITIVE_INFINITY : parsedMax;
  if (isNaN(value) || value === '' || value < 0) {
    return false;
  }

  if (value < min || value > max) {
    return false;
  }

  return true;
}

function validateBetAmountInput(input) {
  if (!validateBetAmount(input)) {
    const min = input.dataset.min;
    const max = input.dataset.max;
    showValidationError(input, min, max);
    return false;
  } else {
    hideValidationError(input);
    return true;
  }
}

function showValidationError(input, min, max) {
  const errorElement = input.parentElement.querySelector('.validation-error');
  const value = parseFloat(input.value);
  const parsedMin = parseFloat(min);
  const numericMin = Number.isNaN(parsedMin) ? 0 : parsedMin;
  const parsedMax = parseFloat(max);
  const numericMax = (max === '' || Number.isNaN(parsedMax)) ? Number.POSITIVE_INFINITY : parsedMax;

  let message = '';
  if (Number.isNaN(value) || value === '' || value < 0) {
    message = 'Please enter a valid positive number';
  } else if (value < numericMin) {
    message = `Amount must be at least ${numericMin}`;
  } else if (Number.isFinite(numericMax) && value > numericMax) {
    message = `Amount cannot exceed ${numericMax}`;
  }

  errorElement.textContent = message;
  errorElement.style.display = 'block';
  input.classList.add('invalid');

  return errorElement;
}

function hideValidationError(input) {
  const errorElement = input.parentElement.querySelector('.validation-error');
  if (errorElement) {
    errorElement.style.display = 'none';
    errorElement.textContent = '';
  }
  input.classList.remove('invalid');
}

// Called from inline onclick handlers in renderBettingControls
// eslint-disable-next-line no-unused-vars
function validateBeforeSubmit(button) {
  const betInput = button.parentElement.querySelector('.bet-input');
  if (betInput && !validateBetAmount(betInput)) {
    validateBetAmountInput(betInput);
    return false;
  }
  return true;
}

/**
 * Action payload construction for htmx
 */
function getActionPayload(actionType, amount) {
  switch (actionType) {
    case 'fold':
      return 'Fold';
    case 'check':
      return 'Check';
    case 'call':
      return 'Call';
    case 'bet':
      return { Bet: amount };
    case 'raise':
      return { Raise: amount };
    case 'all_in':
      return 'AllIn';
    default:
      return null;
  }
}

function getBetAction(actionType) {
  const input = document.querySelector('.bet-input');
  if (!input) return null;

  const amount = parseInt(input.value, 10);
  if (isNaN(amount)) return null;

  return getActionPayload(actionType, amount);
}

/**
 * Hand result display
 */
function renderHandResult(result) {
  const { winner, winner_name, amount, hand_description, showdown, split, fold } = result;

  let winnerDisplay = '';
  if (split) {
    winnerDisplay = '<div class="result-winner split">Split Pot</div>';
  } else if (winner !== null && winner !== undefined) {
    winnerDisplay = `<div class="result-winner">${winner_name || `Player ${winner}`} wins!</div>`;
  }

  const amountDisplay = amount ? `<div class="result-amount">+${amount.toLocaleString()}</div>` : '';
  const descriptionDisplay = hand_description ? `<div class="result-description">${hand_description}</div>` : '';

  let showdownDisplay = '';
  if (showdown && !fold) {
    const player0Cards = showdown.player_0_cards || [];
    const player1Cards = showdown.player_1_cards || [];

    showdownDisplay = `
      <div class="showdown-cards">
        <div class="showdown-player">
          <div class="showdown-label">Your cards:</div>
          <div class="showdown-hand">
            ${player0Cards.map(card => renderCard(card)).join('')}
          </div>
        </div>
        <div class="showdown-player">
          <div class="showdown-label">Opponent's cards:</div>
          <div class="showdown-hand">
            ${player1Cards.map(card => renderCard(card)).join('')}
          </div>
        </div>
      </div>
    `;
  }

  return `
    <div class="hand-result-overlay">
      <div class="hand-result">
        ${winnerDisplay}
        ${amountDisplay}
        ${descriptionDisplay}
        ${showdownDisplay}
        <button class="continue-button" onclick="dismissHandResult()">Continue</button>
      </div>
    </div>
  `;
}

function showHandResult(result) {
  const overlay = document.createElement('div');
  overlay.id = 'hand-result-overlay';
  overlay.innerHTML = renderHandResult(result);
  document.body.appendChild(overlay);
}

function dismissHandResult() {
  const overlay = document.getElementById('hand-result-overlay');
  if (overlay) {
    overlay.remove();
  }
  // Refresh game state to show updated stacks
  refreshGameState();
}

function refreshGameState() {
  const sessionId = getSessionId();
  if (sessionId && typeof htmx !== 'undefined') {
    htmx.ajax('GET', `/api/sessions/${sessionId}/state`, {
      target: '#table',
      swap: 'innerHTML',
      values: { session_id: sessionId }
    });
  }
}

function getSessionId() {
  const tableElement = document.querySelector('[data-session]');
  return tableElement ? tableElement.dataset.session : null;
}

/**
 * SSE Event handling
 */
function setupEventStream(sessionId) {
  const eventSource = new EventSource(`/api/sessions/${sessionId}/events`);

  eventSource.addEventListener('game_event', (event) => {
    const gameEvent = JSON.parse(event.data);
    handleGameEvent(gameEvent);
  });

  eventSource.onerror = (error) => {
    console.error('SSE connection error:', error);
    eventSource.close();
    // Attempt reconnection after delay
    setTimeout(() => {
      if (window.eventSource === eventSource) {
        window.eventSource = setupEventStream(sessionId);
      }
    }, 5000);
  };

  return eventSource;
}

function handleGameEvent(event) {
  console.log('Game event received:', event);

  // Update UI based on event type
  switch (event.type) {
    case 'game_started':
      console.log('Game started with players:', event.players);
      refreshGameState();
      break;
    case 'hand_started':
      console.log('Hand started:', event.hand_id);
      dismissHandResult();
      refreshGameState();
      break;
    case 'cards_dealt':
      console.log('Cards dealt to player', event.player_id);
      refreshGameState();
      break;
    case 'community_cards':
      console.log('Community cards:', event.cards, 'Street:', event.street);
      refreshGameState();
      break;
    case 'player_action':
      console.log('Player action:', event.player_id, event.action);
      refreshGameState();
      break;
    case 'hand_completed':
      console.log('Hand completed:', event.result);
      refreshGameState();
      // Display hand result after short delay to show final state
      setTimeout(() => {
        showHandResult(event.result);
      }, 500);
      break;
    case 'game_ended':
      console.log('Game ended:', event.reason);
      refreshGameState();
      break;
    case 'error':
      console.error('Game error:', event.message);
      break;
  }
}

/**
 * Initialize game controls when DOM is ready
 */
document.addEventListener('DOMContentLoaded', () => {
  console.log('Axiomind Poker game controls initialized');

  // Setup htmx event listeners
  document.body.addEventListener('htmx:afterSwap', (event) => {
    console.log('Content swapped:', event.detail);
  });

  document.body.addEventListener('htmx:responseError', (event) => {
    console.error('Request failed:', event.detail);
    const errorMsg = event.detail.xhr.response;
    alert('Action failed: ' + (errorMsg.message || 'Unknown error'));
  });
});

// Export functions for testing
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    normalizeCard,
    formatCard,
    getCardColor,
    renderCard,
    renderPokerTable,
    renderBettingControls,
    validateBetAmount,
    validateBetAmountInput,
    showValidationError,
    hideValidationError,
    getActionPayload,
    getBetAction,
    renderHandResult,
    showHandResult,
    dismissHandResult,
    refreshGameState,
    getSessionId,
    setupEventStream,
    handleGameEvent
  };
}
