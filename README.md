# Flight PLAN — Linhas Conectoras (plugin After Effects)

Efeito que desenha linhas curvas ("rotas de voo") ligando o contorno (alpha) de um layer
a até 20 nulls, em screen space (respeita câmera e layers 3D).

Feito em Rust com [`after-effects`](https://github.com/virtualritz/after-effects) — não precisa do SDK da Adobe.

## Uso
1. Crie um **sólido 2D do tamanho da comp** e aplique **Effect > Linhas Conectoras > Linhas Conectoras**.
2. **Layer de origem (alpha)**: o texto/sólido cujo contorno gera as linhas.
3. **Nulls**: escolha até 20 nulls.
4. Anime **Animacao > Progresso** (0 → 100%).

**Saida > Debug** mostra cruzes vermelhas nos nulls e pontos amarelos no contorno detectado,
para conferir o alinhamento.

## Instalação (macOS)
1. Em **Actions**, abra a última execução verde e baixe o artefato `LinhasConectoras-mac`.
2. Descompacte e copie `LinhasConectoras.plugin` para
   `/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore/`
3. Libere o plugin (não é notarizado pela Apple):
   ```
   sudo xattr -dr com.apple.quarantine "/Library/Application Support/Adobe/Common/Plug-ins/7.0/MediaCore/LinhasConectoras.plugin"
   ```
4. Reinicie o After Effects.

## Estrutura
- `lc-core/` — geometria, sorteios, detecção de letras, rasterização (testável em qualquer SO).
- `plugin/` — integração com o After Effects (parâmetros, câmera, layers).
- `build_mac.sh` — gera o bundle universal (arm64 + x86_64) com assinatura ad-hoc.
