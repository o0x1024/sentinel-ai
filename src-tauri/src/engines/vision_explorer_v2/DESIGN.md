# Vision Explorer V2 Architecture Design

## 1. Overview
The Vision Explorer V2 represents a paradigm shift from a **Static Partitioning** architecture (Manager/Worker) to a **Layered Event-Driven Actor Model**. It is designed to support "Swarm Intelligence" where specialized agents collaborate to explore web applications.

The core philosophy is the **Hybrid Strategy**: combining the speed and low cost of LLMs (Text/DOM) with the powerful perception of VLMs (Vision).

## 2. Core Architecture

### 2.1 The Brain (Agents)
The system is composed of specialized agents (Actors) communicating via an Event Bus:

*   **PlannerAgent (The Strategist)**
    *   **Role**: The "Hive Brain".
    *   **Responsibility**: Analyzes the `ExplorationGraph`, maintains the "Frontier Queue", and assigns tasks.
    *   **Logic**: Decides *where* to go next based on BFS/DFS or Heuristics. Dispatch tasks to Scout or Specialist pools based on difficulty.

*   **NavigatorAgent (The Mover)**
    *   **Role**: The "Driver".
    *   **Responsibility**: Executes atomic navigation (Goto, Back, Reload). Handles 404s, Redirects, and basic loading states.

*   **AnalystAgent (The Eye)**
    *   An abstract perception layer with two distinct implementations:
    *   **Scout (StructuralAnalyst)**:
        *   **Mode**: LLM (Text/DOM).
        *   **Speed**: High.
        *   **Use Case**: Traversing static links, reading text content, mapping simple site structures.
    *   **Specialist (VisualAnalyst)**:
        *   **Mode**: VLM (Vision).
        *   **Speed**: Low (High Intelligence).
        *   **Use Case**: Analyzing complex SPAs, Canvas elements, solving CAPTCHAs, and handling complex forms.

*   **OperatorAgent (The Hand)**
    *   **Role**: The "Interactor".
    *   **Responsibility**: Performs complex interactions like filling multi-step forms, handling auth flows, and managing file uploads.

*   **NavigationPatternSolver (The Keymaster)**
    *   **Role**: Specialized Agent for complex menus.
    *   **Responsibility**: Solves UX patterns like Hidden Sidebars (Hamburger menus), Accordions, and Parent-Child dependencies in navigation.

### 2.2 The Memory (World Model)
Instead of flat "visited lists", we use a graph-based state system:

*   **ExplorationGraph**
    *   **Nodes**: **Page States**. Defined not just by URL, but by a "State Fingerprint" (DOM Hash + Visual Hash) to support SPAs.
    *   **Edges**: **Actions**. Transitions between states (e.g., `Click(ButtonX)` transitions `StateA` -> `StateB`).
*   **Blackboard**
    *   Shared knowledge base for Auth Tokens, Cookies, Global Configuration, and Safety Policies.

### 2.3 The Body (Driver)
*   **BrowserDriver**: A stateless, thin wrapper around Playwright/CDP. It has no business logic, only execution capabilities.

## 3. Hybrid Exploration Strategy

The system dynamically switches modes based on the context:

1.  **Fast-Pass (Default)**: The `Planner` assigns tasks to **Scout Agents**. They scan the DOM, extract links, and build the graph skeleton quickly.
2.  **Deep-Dive (On-Demand)**: If a Scout encounters a "black box" (e.g., `<canvas>`, empty DOM with visual content) or a complex heuristic (e.g., "Login", "Payment"), the Planner marks the node as "Complex". A **Specialist Agent (VLM)** is then dispatched to visually analyze and interact with it.
3.  **Visual Cues**: The system actively scans for visual navigation patterns (e.g., Hamburger icons) that might be missed by DOM analysis.

## 4. Addressing Complex Navigation
The V2 Architecture specifically targets difficult UI patterns (as identified in user feedback):

*   **Parent-Child Dependencies**: Modeled as graph dependencies. The Planner understands that Node B (Child Menu) is only reachable from Node A (Parent Menu) via a specific Action (Expand).
*   **Hidden/Hover Menus**: `VisualAnalyst` uses VLM to identify "Menu" icons and "Hover" triggers, creating explicit "Reveal" actions in the graph.
*   **Accordion/Collapsed States**: The graph treats "Collapsed" and "Expanded" as two distinct Nodes connected by a "Toggle" edge, ensuring full coverage.

## 5. Configuration Transformation (UI Changes)

To support this architecture, the **AI Settings** (`src/components/Settings/AISettings.vue`) must be overhauled:

| Current Setting | New Setting | Description |
| :--- | :--- | :--- |
| `Default Provider/Model` | **Default Fast Model (LLM)** | Used by **Scout Agents**. Should be a fast, cheap model (e.g., Haiku, Flash, 4o-mini). |
| *N/A* | **Default Vision Model (VLM)** | **[NEW]** Used by **Specialist Agents**. Must be a high-intelligence vision model (e.g., Sonnet 3.5, GPT-4o). |
| `Enable Multimodal` (Toggle) | *Removed* | The system is now natively hybrid. The toggle is obsolete. |

### Migration Steps for Settings
1.  Add `defaultVlmModel` selection field.
2.  Remove `enableMultimodal` checkbox.
3.  Update configuration interfaces/structs to pass both model configs to the V2 Engine.
