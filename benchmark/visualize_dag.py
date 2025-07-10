import json
import networkx as nx
import matplotlib.pyplot as plt
from matplotlib.patches import Patch
from collections import defaultdict

# 读取 committee，建立 author->validator 的映射
with open('/home/ccclr0302/autobahn-test/benchmark/.committee.json', 'r') as f:
    committee = json.load(f)

# 取出所有 authorities 的 key（完整 base64 公钥）
authority_keys = list(committee['authorities'].keys())

# 读取 DAG 快照
with open('/home/ccclr0302/dag_snapshot_slot21_view1.json', 'r') as f:
    data = json.load(f)

nodes = data['nodes']


# 建立 author（短名）到 validator（完整key）的映射
author_map = {}
for short in set(n['author'] for n in nodes):
    for full in authority_keys:
        if short in full:
            author_map[short] = full
            break
    else:
        author_map[short] = short  # fallback

# 统计所有 validator（按 committee 顺序）
validators = [k for k in authority_keys]
validator_pos = {v: i for i, v in enumerate(validators)}


# === Step 1: 找出每个 validator 的最高高度节点（active tip） ===
validator_latest = {}  # validator -> (height, digest)

for node in nodes:
    v = author_map.get(node['author'], node['author'])
    h = node['height']
    digest = node['digest']
    if v not in validator_latest or h > validator_latest[v][0]:
        validator_latest[v] = (h, digest)

active_tips = set(d for (_, d) in validator_latest.values())


# 统计所有 round（height）的绝对值
heights = sorted(set(n['height'] for n in nodes))

G = nx.DiGraph()
for node in nodes:
    digest = node['digest']
    author = node['author']
    height = node['height']
    validator = author_map.get(author, author)
    G.add_node(digest, validator=validator, round=height)
    for parent in node['parents']:
        G.add_edge(digest, parent)

# 计算节点坐标：x=height绝对值, y=validator序号
pos = {}
for node, attr in G.nodes(data=True):
    if 'validator' not in attr or 'round' not in attr:
        continue  # 跳过没有属性的节点
    v = attr['validator']
    r = attr['round']
    x = r
    y = -validator_pos.get(v, 0)
    pos[node] = (x, y)

plt.figure(figsize=(max(8, len(heights)), max(6, len(validators))))

node_colors = []
for node in pos:
    if node in active_tips:
        node_colors.append("orange")  # active tip
    elif G.out_degree(node) == 0:
        node_colors.append("gray")    # orphaned tip
    else:
        node_colors.append("white")   # internal node

nx.draw_networkx_nodes(
    G, pos,
    nodelist=list(pos.keys()),
    node_size=300,
    node_color=node_colors,
    edgecolors='black',
    linewidths=1
)


# 画有箭头的边
edges_to_draw = [(u, v) for u, v in G.edges() if u in pos and v in pos]
nx.draw_networkx_edges(
    G, pos,
    edgelist=edges_to_draw,
    arrows=True,
    arrowstyle='-|>',
    arrowsize=18,
    edge_color='gray',
    width=1.5,
    connectionstyle='arc3,rad=0.08',
    min_source_margin=10,
    min_target_margin=10,
    style='-'
)

# 画 cut 线
last_cut_tips = data.get('last_cut_tips', {})
cut_points = []
for v, h in last_cut_tips.items():
    if v in validator_pos:
        x = h  # height 绝对值
        y = -validator_pos[v]
        cut_points.append((x, y))
if cut_points:
    # 按 y 排序（可选，也可以按 x 排序）
    cut_points = sorted(cut_points, key=lambda p: p[1])
    xs, ys = zip(*cut_points)
    plt.plot(xs, ys, color='blue', linewidth=2, marker='o', markersize=10, label='cut line')

# 标注 height（绝对值）
for h in heights:
    plt.text(h, 1, f"h = {h}", ha='center', va='bottom', fontsize=10)
# 标注 validator
for v, y in validator_pos.items():
    plt.text(min(heights)-1, -y, v[:8], ha='right', va='center', fontsize=10)

# legend_elements = [
#     Patch(facecolor='orange', edgecolor='black', label='Active Tip (used for cut)'),
#     Patch(facecolor='gray', edgecolor='black', label='Orphaned Tip (not used)'),
#     Patch(facecolor='white', edgecolor='black', label='Internal Node'),
#     Patch(color='blue', label='Cut Line')
# ]
# plt.legend(handles=legend_elements, loc='lower left')
plt.axis('off')
plt.title(f"DAG Snapshot (slot {data['committed_slot']}, view {data['view']})")
plt.tight_layout()
plt.savefig('dag.png')